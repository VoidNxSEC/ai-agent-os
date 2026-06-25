use anyhow::Result;
use spectre_core::ServiceId;
use spectre_events::{Event, EventType};
use tracing::warn;

use crate::{EmailItem, Temperature};

/// Publish an `email.intelligence.v1` event to Spectre (best-effort).
pub async fn publish_email_event(nats: &async_nats::Client, item: &EmailItem) -> Result<()> {
    let temp_str = match item.temperature {
        Temperature::Hot => "hot",
        Temperature::Warm => "warm",
        Temperature::Cold => "cold",
    };

    let event = Event::new(
        EventType::Custom("email.intelligence.v1".to_string()),
        ServiceId::new("email-monitor"),
        serde_json::json!({
            "from":        item.from,
            "subject":     item.subject,
            "reason":      item.reason,
            "temperature": temp_str,
        }),
    );

    let json = event.to_json().map_err(|e| anyhow::anyhow!("{}", e))?;
    nats.publish("email.intelligence.v1", json.into()).await?;
    Ok(())
}

/// Fire-and-forget: publish to Spectre, log on failure, never abort.
pub async fn try_publish(nats: &async_nats::Client, item: &EmailItem) {
    if let Err(e) = publish_email_event(nats, item).await {
        warn!("Spectre publish email.intelligence.v1 failed: {}", e);
    }
}
