//! AI Agent OS - Main binary
//!
//! Example usage of the agent core library

use agent_core::{Agent, AgentConfig, PhantomGateConfig};
use anyhow::Result;
use tracing::{info, Level};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("🤖 AI Agent OS - Starting...");

    // Create agent with default config
    let config = AgentConfig {
        monitoring_interval_secs: 5,
        thermal_threshold_celsius: 75.0,
        memory_threshold_percent: 85.0,
        enable_hyprland: true,
        phantom_gate: PhantomGateConfig {
            enabled: true,
            ..Default::default()
        },
        ..Default::default()
    };

    let agent = Agent::with_config(config)?;

    info!("✅ Agent initialized successfully");

    // Get initial state
    let state = agent.get_state().await;
    info!(
        "📊 Initial state: running={}, hyprland_connected={}",
        state.running, state.hyprland_connected
    );

    // Get current metrics
    match agent.get_metrics().await {
        Ok(metrics) => {
            info!("📈 System Metrics:");
            info!(
                "  CPU: {:.1}% (cores: {})",
                metrics.cpu.usage_percent,
                metrics.cpu.cores.len()
            );
            info!(
                "  Memory: {:.1}% ({} MB / {} MB)",
                metrics.memory.usage_percent,
                metrics.memory.used_bytes / (1024 * 1024),
                metrics.memory.total_bytes / (1024 * 1024)
            );
            info!(
                "  Temperature: {:.1}°C (max: {:.1}°C)",
                metrics.thermal.avg_temp_celsius, metrics.thermal.max_temp_celsius
            );
            info!("  Disks: {} mounted", metrics.disk.disks.len());
        }
        Err(e) => {
            info!("⚠️  Failed to get metrics: {}", e);
        }
    }

    // Get recent logs
    match agent.get_logs(5).await {
        Ok(logs) => {
            info!("📜 Recent system logs ({} entries):", logs.len());
            for log in logs.iter().take(5) {
                info!(
                    "  [{:?}] {}",
                    log.priority,
                    log.message.chars().take(100).collect::<String>()
                );
            }
        }
        Err(e) => {
            info!("⚠️  Failed to get logs: {}", e);
        }
    }

    info!("🚀 Starting agent monitoring...");
    info!("   Press Ctrl+C to stop");

    // Start the agent (this will run until interrupted)
    tokio::select! {
        result = agent.start() => {
            match result {
                Ok(_) => info!("Agent stopped normally"),
                Err(e) => info!("Agent error: {}", e),
            }
        }
        _ = tokio::signal::ctrl_c() => {
            info!("🛑 Received shutdown signal");
            agent.stop().await?;
        }
    }

    info!("👋 AI Agent OS stopped");

    Ok(())
}
