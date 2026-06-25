use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use agent_core::{Alert, AlertCategory, AlertSeverity, AlertSystem, Monitor};
use anyhow::Result;
use notify_rust::{Notification, Timeout, Urgency};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

pub mod spectre;

// ─── types ────────────────────────────────────────────────────────────────────

/// Maddy local MTA connection parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaddyConfig {
    /// Maddy IMAP host — almost always 127.0.0.1 when co-located.
    pub host: String,
    /// IMAP port. Maddy default is 143 (plain) or 993 (TLS).
    pub port: u16,
    pub user: String,
    pub password: String,
    /// IMAP mailbox to monitor. Default: "INBOX".
    pub mailbox: String,
}

impl Default for MaddyConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 143,
            user: String::new(),
            password: String::new(),
            mailbox: "INBOX".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Temperature {
    Hot,
    Warm,
    Cold,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailItem {
    pub from: String,
    pub subject: String,
    pub reason: String,
    pub temperature: Temperature,
}

impl EmailItem {
    pub fn is_hot(&self) -> bool {
        self.temperature == Temperature::Hot
    }
    pub fn is_warm(&self) -> bool {
        self.temperature == Temperature::Warm
    }
}

/// Waybar JSON payload (see skill §IV).
#[derive(Serialize)]
struct WaybarPayload {
    text: String,
    tooltip: String,
    class: String,
}

// ─── monitor ──────────────────────────────────────────────────────────────────

pub struct EmailMonitor {
    pub poll_interval_secs: u64,
    pub anthropic_api_key: String,
    pub maddy: MaddyConfig,
    pub hot_only_notify: bool,
    http: reqwest::Client,
}

impl EmailMonitor {
    pub fn new(
        poll_interval_secs: u64,
        anthropic_api_key: impl Into<String>,
        maddy: MaddyConfig,
        hot_only_notify: bool,
    ) -> Self {
        Self {
            poll_interval_secs,
            anthropic_api_key: anthropic_api_key.into(),
            maddy,
            hot_only_notify,
            http: reqwest::Client::new(),
        }
    }

    /// Fetch unseen messages from Maddy and classify each via Claude Haiku.
    pub async fn fetch_and_classify(&self) -> Result<Vec<EmailItem>> {
        let maddy = self.maddy.clone();

        // IMAP is synchronous — run in blocking thread.
        let headers: Vec<(String, String)> =
            tokio::task::spawn_blocking(move || imap_fetch_unseen(&maddy))
                .await
                .map_err(|e| anyhow::anyhow!("IMAP task panicked: {}", e))??;

        if headers.is_empty() {
            debug!("No unseen messages in {}", self.maddy.mailbox);
            return Ok(vec![]);
        }

        info!("Classifying {} unseen message(s) via Claude Haiku", headers.len());

        let mut items = Vec::with_capacity(headers.len());
        for (from, subject) in headers {
            match classify_email(&self.http, &self.anthropic_api_key, &from, &subject).await {
                Ok((temperature, reason)) => items.push(EmailItem {
                    from,
                    subject,
                    reason,
                    temperature,
                }),
                Err(e) => warn!("Classification failed for '{}': {}", subject, e),
            }
        }

        Ok(items)
    }
}

impl Monitor for EmailMonitor {
    fn run(
        &self,
        alerts: Arc<AlertSystem>,
    ) -> impl std::future::Future<Output = Result<()>> + Send + '_ {
        async move {
            info!(
                "EmailMonitor started — Maddy {}:{}, poll every {}s",
                self.maddy.host, self.maddy.port, self.poll_interval_secs
            );

            loop {
                match self.fetch_and_classify().await {
                    Ok(emails) => {
                        let hot_count = emails.iter().filter(|e| e.is_hot()).count();

                        if let Err(e) = update_waybar(hot_count, &emails) {
                            warn!("Waybar update failed: {}", e);
                        }

                        for email in emails.iter().filter(|e| e.is_hot()) {
                            notify_hot(&email.subject, &email.reason).await;

                            alerts
                                .send(Alert {
                                    timestamp: unix_now(),
                                    severity: AlertSeverity::Critical,
                                    category: AlertCategory::Email,
                                    message: format!("🔥 {}", email.subject),
                                    details: Some(format!(
                                        "{} — {}",
                                        email.from, email.reason
                                    )),
                                })
                                .await;

                            if let Some(ref nats) = alerts.nats_client {
                                spectre::try_publish(nats, email).await;
                            }
                        }

                        if !self.hot_only_notify {
                            for email in emails.iter().filter(|e| e.is_warm()) {
                                notify_warm(&email.subject, &email.reason).await;

                                if let Some(ref nats) = alerts.nats_client {
                                    spectre::try_publish(nats, email).await;
                                }
                            }
                        }
                    }
                    Err(e) => warn!("EmailMonitor poll error: {}", e),
                }

                tokio::time::sleep(Duration::from_secs(self.poll_interval_secs)).await;
            }
        }
    }
}

// ─── IMAP (sync, runs in spawn_blocking) ─────────────────────────────────────

fn imap_fetch_unseen(config: &MaddyConfig) -> Result<Vec<(String, String)>> {
    let addr = (config.host.as_str(), config.port);

    // Plain TCP — Maddy is always co-located on localhost, no TLS needed.
    let stream = std::net::TcpStream::connect(addr)
        .map_err(|e| anyhow::anyhow!("Cannot connect to Maddy at {}:{}: {}", config.host, config.port, e))?;
    let client = imap::Client::new(stream);

    let mut session = client
        .login(&config.user, &config.password)
        .map_err(|(err, _)| anyhow::anyhow!("IMAP login failed: {}", err))?;

    session.select(&config.mailbox)?;

    let uids = session.uid_search("UNSEEN")?;
    if uids.is_empty() {
        session.logout()?;
        return Ok(vec![]);
    }

    // Cap at 20 per poll to avoid bursts.
    let uid_str = uids
        .iter()
        .take(20)
        .map(|u| u.to_string())
        .collect::<Vec<_>>()
        .join(",");

    let messages = session.uid_fetch(uid_str, "RFC822.HEADER")?;
    let mut results = Vec::new();

    for msg in messages.iter() {
        if let Some(raw) = msg.header() {
            let header = std::str::from_utf8(raw).unwrap_or("");
            let from = extract_header(header, "From").unwrap_or_default();
            let subject = extract_header(header, "Subject").unwrap_or_default();
            if !from.is_empty() || !subject.is_empty() {
                results.push((from, subject));
            }
        }
    }

    session.logout()?;
    Ok(results)
}

/// Extract a single-line header value (case-insensitive name match).
fn extract_header(headers: &str, name: &str) -> Option<String> {
    let lower_name = name.to_lowercase();
    let prefix = format!("{}:", lower_name);
    for line in headers.lines() {
        if line.to_lowercase().starts_with(&prefix) {
            return Some(line[name.len() + 1..].trim().to_string());
        }
    }
    None
}

// ─── Anthropic classification ─────────────────────────────────────────────────

async fn classify_email(
    http: &reqwest::Client,
    api_key: &str,
    from: &str,
    subject: &str,
) -> Result<(Temperature, String)> {
    #[derive(Serialize)]
    struct Body {
        model: String,
        max_tokens: u32,
        system: String,
        messages: Vec<Message>,
    }
    #[derive(Serialize)]
    struct Message {
        role: String,
        content: String,
    }

    let body = Body {
        model: "claude-haiku-4-5-20251001".to_string(),
        max_tokens: 80,
        system: concat!(
            "Classify this email as hot, warm, or cold.\n",
            "hot  = requires action today (interview, urgent client, deadline, security alert)\n",
            "warm = worth reading but not urgent\n",
            "cold = newsletter, marketing, or irrelevant\n",
            "Reply with exactly two lines:\n",
            "TEMPERATURE: <hot|warm|cold>\n",
            "REASON: <one concise sentence>"
        )
        .to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: format!("From: {}\nSubject: {}", from, subject),
        }],
    };

    let resp = http
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await?
        .error_for_status()?;

    let json: serde_json::Value = resp.json().await?;
    let text = json["content"][0]["text"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();

    parse_classification(&text)
}

fn parse_classification(text: &str) -> Result<(Temperature, String)> {
    let mut temperature = Temperature::Cold;
    let mut reason = "no reason provided".to_string();

    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("TEMPERATURE:") {
            temperature = match rest.trim().to_lowercase().as_str() {
                "hot" => Temperature::Hot,
                "warm" => Temperature::Warm,
                _ => Temperature::Cold,
            };
        } else if let Some(rest) = line.strip_prefix("REASON:") {
            reason = rest.trim().to_string();
        }
    }

    Ok((temperature, reason))
}

// ─── notifications (skill §III) ──────────────────────────────────────────────

async fn notify_hot(summary: &str, body: &str) {
    let summary = summary.to_string();
    let body = body.to_string();
    tokio::task::spawn_blocking(move || {
        if let Err(e) = Notification::new()
            .summary(&summary)
            .body(&body)
            .urgency(Urgency::Critical)
            .timeout(Timeout::Milliseconds(8000))
            .hint(notify_rust::Hint::Custom(
                "x-canonical-private-synchronous".to_string(),
                "voidnx-hot".to_string(),
            ))
            .show()
        {
            warn!("notify_hot failed: {}", e);
        }
    })
    .await
    .ok();
}

async fn notify_warm(summary: &str, body: &str) {
    let summary = summary.to_string();
    let body = body.to_string();
    tokio::task::spawn_blocking(move || {
        if let Err(e) = Notification::new()
            .summary(&summary)
            .body(&body)
            .urgency(Urgency::Normal)
            .timeout(Timeout::Milliseconds(4000))
            .show()
        {
            warn!("notify_warm failed: {}", e);
        }
    })
    .await
    .ok();
}

// ─── waybar (skill §IV) ───────────────────────────────────────────────────────

fn update_waybar(hot_count: usize, items: &[EmailItem]) -> Result<()> {
    let class = if hot_count > 0 { "hot" } else { "idle" };
    let text = if hot_count > 0 {
        format!("🔥 {}", hot_count)
    } else {
        "—".to_string()
    };

    let tooltip = items
        .iter()
        .filter(|e| e.is_hot())
        .map(|e| format!("{}\n{}", e.from, e.reason))
        .collect::<Vec<_>>()
        .join("\n\n");

    let payload = WaybarPayload {
        text,
        tooltip,
        class: class.to_string(),
    };

    let json = serde_json::to_string(&payload)?;
    std::fs::write("/tmp/voidnx-inbox.json", json)?;
    Ok(())
}

// ─── utils ────────────────────────────────────────────────────────────────────

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
