//! Core agent module that orchestrates system monitoring, log collection, and Hyprland integration
//!
//! This is the main entry point for the AI Agent OS, coordinating all subsystems
//! and providing a unified interface for monitoring and control.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use spectre_core::ServiceId;
use spectre_events::{Event, EventType};

mod phantom_gate;
pub use phantom_gate::{PhantomGate, PhantomGateBundle, PhantomGateConfig, PhantomGateResult};

// Re-export crate modules
pub use hyprland_ipc;
pub use log_collector;
pub use system_monitor;

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub monitoring_interval_secs: u64,
    pub thermal_threshold_celsius: f32,
    pub memory_threshold_percent: f32,
    pub enable_hyprland: bool,
    pub log_filter: log_collector::LogFilter,
    pub phantom_gate: PhantomGateConfig,
    /// Optional NATS URL for Spectre event publishing.
    /// If None, event publishing is disabled (agent runs standalone).
    pub nats_url: Option<String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            monitoring_interval_secs: 5,
            thermal_threshold_celsius: 75.0,
            memory_threshold_percent: 85.0,
            enable_hyprland: true,
            log_filter: log_collector::LogFilter::default(),
            phantom_gate: PhantomGateConfig::default(),
            nats_url: None,
        }
    }
}

/// Agent state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub running: bool,
    pub last_metrics: Option<system_monitor::SystemMetrics>,
    pub alerts: Vec<Alert>,
    pub hyprland_connected: bool,
}

/// System alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub timestamp: u64,
    pub severity: AlertSeverity,
    pub category: AlertCategory,
    pub message: String,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertCategory {
    Thermal,
    Memory,
    Disk,
    Network,
    System,
    Hyprland,
}

/// Main agent orchestrator
pub struct Agent {
    config: AgentConfig,
    state: Arc<RwLock<AgentState>>,
    system_monitor: Arc<RwLock<system_monitor::SystemMonitor>>,
    hyprland_client: Option<Arc<RwLock<hyprland_ipc::HyprlandClient>>>,
    phantom_gate: Option<Arc<PhantomGate>>,
    /// NATS client for Spectre event publishing (None = disabled).
    nats_client: Option<Arc<async_nats::Client>>,
}

impl Agent {
    /// Create a new agent with default configuration
    pub async fn new() -> Result<Self> {
        Self::with_config(AgentConfig::default()).await
    }

    /// Create a new agent with custom configuration
    pub async fn with_config(config: AgentConfig) -> Result<Self> {
        info!("Initializing AI Agent OS...");

        let system_monitor = Arc::new(RwLock::new(system_monitor::SystemMonitor::new()));

        let hyprland_client = if config.enable_hyprland {
            match hyprland_ipc::HyprlandClient::new() {
                Ok(client) => {
                    info!("Hyprland IPC connected");
                    Some(Arc::new(RwLock::new(client)))
                }
                Err(e) => {
                    warn!("Failed to connect to Hyprland: {}. Continuing without Hyprland integration.", e);
                    None
                }
            }
        } else {
            None
        };

        let state = Arc::new(RwLock::new(AgentState {
            running: false,
            last_metrics: None,
            alerts: Vec::new(),
            hyprland_connected: hyprland_client.is_some(),
        }));

        let phantom_gate = if config.phantom_gate.enabled {
            match PhantomGate::new(config.phantom_gate.clone()) {
                Ok(g) => Some(Arc::new(g)),
                Err(e) => {
                    warn!("Failed to initialize PhantomGate: {}. Continuing without Phantom integration.", e);
                    None
                }
            }
        } else {
            None
        };

        // Connect to NATS if configured (best-effort: failure doesn't abort startup).
        // Uses ConnectOptions with unlimited reconnect so ai-agent-os survives
        // transient NATS restarts and resumes publishing automatically.
        let nats_client = if let Some(ref url) = config.nats_url {
            let opts = async_nats::ConnectOptions::new()
                .max_reconnects(None) // unlimited
                .connection_timeout(std::time::Duration::from_secs(5))
                .event_callback(|event| async move {
                    match event {
                        async_nats::Event::Disconnected => {
                            warn!("Spectre NATS disconnected — will reconnect automatically");
                        }
                        async_nats::Event::Connected => {
                            info!("Spectre NATS connected/reconnected");
                        }
                        async_nats::Event::ClientError(err) => {
                            warn!("Spectre NATS client error: {}", err);
                        }
                        _ => {}
                    }
                });
            match opts.connect(url).await {
                Ok(client) => {
                    info!("Spectre NATS connected to {}", url);
                    Some(Arc::new(client))
                }
                Err(e) => {
                    warn!("Failed to connect to NATS at {}: {}. Event publishing disabled.", url, e);
                    None
                }
            }
        } else {
            None
        };

        Ok(Self {
            config,
            state,
            system_monitor,
            hyprland_client,
            phantom_gate,
            nats_client,
        })
    }

    /// Start the agent
    pub async fn start(&self) -> Result<()> {
        info!("Starting AI Agent OS...");

        {
            let mut state = self.state.write().await;
            state.running = true;
        }

        // Spawn monitoring task
        let monitor_handle = self.spawn_monitoring_task(self.nats_client.clone());

        // Spawn Hyprland event listener if available
        let hyprland_handle = if self.hyprland_client.is_some() {
            Some(self.spawn_hyprland_task())
        } else {
            None
        };

        info!("AI Agent OS started successfully");

        // Wait for tasks (in a real application, you'd want better task management)
        if let Some(hyprland_handle) = hyprland_handle {
            tokio::select! {
                _ = monitor_handle => warn!("Monitoring task ended"),
                _ = hyprland_handle => warn!("Hyprland task ended"),
            }
        } else {
            monitor_handle.await.ok();
        }

        Ok(())
    }

    /// Stop the agent
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping AI Agent OS...");

        let mut state = self.state.write().await;
        state.running = false;

        Ok(())
    }

    /// Get current state
    pub async fn get_state(&self) -> AgentState {
        self.state.read().await.clone()
    }

    /// Get current system metrics
    pub async fn get_metrics(&self) -> Result<system_monitor::SystemMetrics> {
        let mut monitor = self.system_monitor.write().await;
        monitor.collect()
    }

    /// Get recent log entries (creates collector on demand)
    pub async fn get_logs(&self, count: usize) -> Result<Vec<log_collector::LogEntry>> {
        tokio::task::spawn_blocking(move || {
            let mut collector = log_collector::LogCollector::new()?;
            collector.get_recent_entries(count)
        })
        .await
        .context("Log collection task panicked")?
    }

    /// Spawn monitoring task
    fn spawn_monitoring_task(
        &self,
        nats_client: Option<Arc<async_nats::Client>>,
    ) -> tokio::task::JoinHandle<()> {
        let state = Arc::clone(&self.state);
        let monitor = Arc::clone(&self.system_monitor);
        let interval = self.config.monitoring_interval_secs;
        let thermal_threshold = self.config.thermal_threshold_celsius;
        let memory_threshold = self.config.memory_threshold_percent;
        let phantom_gate = self.phantom_gate.clone();

        tokio::spawn(async move {
            info!("System monitoring task started");

            loop {
                // Check if we should stop
                {
                    let state_guard = state.read().await;
                    if !state_guard.running {
                        break;
                    }
                }

                // Collect metrics
                let metrics = {
                    let mut monitor_guard = monitor.write().await;
                    match monitor_guard.collect() {
                        Ok(m) => m,
                        Err(e) => {
                            error!("Failed to collect metrics: {}", e);
                            tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
                            continue;
                        }
                    }
                };

                // Publish system.metrics.v1 to Spectre (best-effort)
                if let Some(ref nc) = nats_client {
                    let payload = serde_json::json!({
                        "cpu_percent": metrics.cpu.usage_percent,
                        "memory_percent": metrics.memory.usage_percent,
                        "memory_used_bytes": metrics.memory.used_bytes,
                        "memory_total_bytes": metrics.memory.total_bytes,
                        "temp_avg_celsius": metrics.thermal.avg_temp_celsius,
                        "temp_max_celsius": metrics.thermal.max_temp_celsius,
                        "disk_count": metrics.disk.disks.len(),
                        "net_rx_bytes": metrics.network.total_rx_bytes,
                        "net_tx_bytes": metrics.network.total_tx_bytes,
                    });
                    let event = Event::new(
                        EventType::SystemMetrics,
                        ServiceId::new("ai-agent-os"),
                        payload,
                    );
                    match event.to_json() {
                        Ok(json) => {
                            let subject = event.subject();
                            if let Err(e) = nc.publish(subject, json.into()).await {
                                debug!("NATS publish failed: {}", e);
                            }
                        }
                        Err(e) => debug!("Failed to serialize metrics event: {}", e),
                    }
                }

                // Check for alerts
                let mut alerts = Vec::new();

                // Thermal check
                if metrics.thermal.max_temp_celsius > thermal_threshold {
                    alerts.push(Alert {
                        timestamp: metrics.timestamp,
                        severity: AlertSeverity::Warning,
                        category: AlertCategory::Thermal,
                        message: format!(
                            "High temperature detected: {:.1}°C",
                            metrics.thermal.max_temp_celsius
                        ),
                        details: Some(format!(
                            "Threshold: {:.1}°C, Sensors: {}",
                            thermal_threshold,
                            metrics.thermal.sensors.len()
                        )),
                    });
                }

                // Memory check
                if metrics.memory.usage_percent > memory_threshold {
                    alerts.push(Alert {
                        timestamp: metrics.timestamp,
                        severity: AlertSeverity::Warning,
                        category: AlertCategory::Memory,
                        message: format!("High memory usage: {:.1}%", metrics.memory.usage_percent),
                        details: Some(format!(
                            "Used: {} MB, Total: {} MB",
                            metrics.memory.used_bytes / (1024 * 1024),
                            metrics.memory.total_bytes / (1024 * 1024)
                        )),
                    });
                }

                // Update state
                {
                    let mut state_guard = state.write().await;
                    state_guard.last_metrics = Some(metrics.clone());
                    state_guard.alerts.extend(alerts.clone());

                    // Keep only last 100 alerts
                    if state_guard.alerts.len() > 100 {
                        let drain_count = state_guard.alerts.len() - 100;
                        state_guard.alerts.drain(0..drain_count);
                    }
                }

                // Log alerts
                for alert in alerts.clone() {
                    match alert.severity {
                        AlertSeverity::Info => info!("{}", alert.message),
                        AlertSeverity::Warning => warn!("{}", alert.message),
                        AlertSeverity::Critical => error!("{}", alert.message),
                    }
                }

                // Phantom judging pipeline: produce a report bundle and let Phantom classify/sanitize/audit it.
                // Best-effort + report-only: failures should not break monitoring.
                if let Some(gate) = phantom_gate.clone() {
                    if !alerts.is_empty() {
                        let metrics_copy = metrics.clone();
                        let alerts_copy = alerts.clone();

                        tokio::spawn(async move {
                            // Collect recent logs (best-effort)
                            let logs: Vec<log_collector::LogEntry> =
                                tokio::task::spawn_blocking(|| {
                                    let mut collector = log_collector::LogCollector::new()?;
                                    collector.get_recent_entries(200)
                                })
                                .await
                                .ok()
                                .and_then(
                                    |r: Result<Vec<log_collector::LogEntry>, anyhow::Error>| r.ok(),
                                )
                                .unwrap_or_default();

                            let hostname = std::env::var("HOSTNAME").ok();

                            let bundle = PhantomGateBundle {
                                timestamp: metrics_copy.timestamp,
                                hostname,
                                metrics: metrics_copy,
                                alerts: alerts_copy,
                                logs,
                            };

                            match gate.judge_bundle(&bundle).await {
                                Ok(res) => {
                                    info!(
                                        "🔮 PhantomGate judged bundle: dir={} bundle={}",
                                        res.bundle_dir, res.bundle_file
                                    );
                                    for note in res.notes {
                                        warn!("PhantomGate note: {}", note);
                                    }
                                }
                                Err(e) => warn!("PhantomGate failed: {}", e),
                            }
                        });
                    }
                }

                debug!(
                    "Metrics collected - CPU: {:.1}%, Memory: {:.1}%, Temp: {:.1}°C",
                    metrics.cpu.usage_percent,
                    metrics.memory.usage_percent,
                    metrics.thermal.max_temp_celsius
                );

                tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
            }

            info!("System monitoring task stopped");
        })
    }

    /// Spawn Hyprland event listener task
    fn spawn_hyprland_task(&self) -> tokio::task::JoinHandle<()> {
        let state = Arc::clone(&self.state);
        let client = self.hyprland_client.as_ref().unwrap().clone();

        tokio::spawn(async move {
            info!("Hyprland event listener started");

            // Subscribe to events
            let mut event_stream = match client.read().await.subscribe_events().await {
                Ok(stream) => stream,
                Err(e) => {
                    error!("Failed to subscribe to Hyprland events: {}", e);
                    return;
                }
            };

            loop {
                // Check if we should stop
                {
                    let state_guard = state.read().await;
                    if !state_guard.running {
                        break;
                    }
                }

                // Read next event
                match event_stream.next_event().await {
                    Ok(Some(event)) => {
                        debug!("Hyprland event: {:?}", event);

                        // You could process events here and create alerts
                        // For example, detect when certain windows open/close
                    }
                    Ok(None) => {
                        // No event, continue
                    }
                    Err(e) => {
                        error!("Failed to read Hyprland event: {}", e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }
                }
            }

            info!("Hyprland event listener stopped");
        })
    }
}

// Note: Default is not implemented for Agent because construction is async.
// Use Agent::new().await or Agent::with_config(config).await.

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_creation() {
        let agent = Agent::new().await;
        assert!(agent.is_ok());
    }

    #[tokio::test]
    async fn test_agent_state() {
        let agent = Agent::new().await.unwrap();
        let state = agent.get_state().await;
        assert!(!state.running);
    }
}
