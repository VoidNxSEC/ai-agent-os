//! Tauri App Library - GUI Integration with AI Intelligence
//!
//! Connects the autonomous AI agent to a native GUI interface.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{Manager, State, Window};
use tokio::sync::RwLock;

use ai_intelligence::{AgentState, IntelligentAgent, SystemAnalysis};

/// Global agent state
pub struct AppState {
    pub agent: Arc<RwLock<Option<IntelligentAgent>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            agent: Arc::new(RwLock::new(None)),
        }
    }
}

/// System metrics for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu_percent: f32,
    pub memory_percent: f32,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
    pub temp_celsius: f32,
    pub disk_percent: f32,
    pub uptime_seconds: u64,
}

/// Problem notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemNotification {
    pub severity: String,
    pub title: String,
    pub message: String,
    pub timestamp: String,
}

/// Tauri command: Initialize the intelligent agent
#[tauri::command]
pub async fn init_agent(state: State<'_, AppState>) -> Result<String, String> {
    tracing::info!("🚀 Initializing Intelligent Agent...");

    match IntelligentAgent::new().await {
        Ok(agent) => {
            *state.agent.write().await = Some(agent);
            Ok("Agent initialized successfully".to_string())
        }
        Err(e) => {
            tracing::error!("Failed to initialize agent: {}", e);
            Err(format!("Failed to initialize agent: {}", e))
        }
    }
}

/// Tauri command: Get current system metrics
#[tauri::command]
pub async fn get_metrics(state: State<'_, AppState>) -> Result<SystemMetrics, String> {
    // TODO: Integrate with actual system-monitor
    // For now, return mock data
    Ok(SystemMetrics {
        cpu_percent: 45.2,
        memory_percent: 62.8,
        memory_used_mb: 9856,
        memory_total_mb: 15698,
        temp_celsius: 67.3,
        disk_percent: 58.9,
        uptime_seconds: 123456,
    })
}

/// Tauri command: Get agent state
#[tauri::command]
pub async fn get_agent_state(state: State<'_, AppState>) -> Result<AgentState, String> {
    let agent_lock = state.agent.read().await;

    if let Some(agent) = agent_lock.as_ref() {
        Ok(agent.get_state().await)
    } else {
        Err("Agent not initialized".to_string())
    }
}

/// Tauri command: Request system analysis
#[tauri::command]
pub async fn analyze_system(state: State<'_, AppState>) -> Result<SystemAnalysis, String> {
    let agent_lock = state.agent.read().await;

    if let Some(agent) = agent_lock.as_ref() {
        agent
            .analyze_now()
            .await
            .map_err(|e| format!("Analysis failed: {}", e))
    } else {
        Err("Agent not initialized".to_string())
    }
}

/// Tauri command: Execute a command
#[tauri::command]
pub async fn execute_command(command: String) -> Result<String, String> {
    tracing::info!("Executing command: {}", command);

    // Execute command safely
    let output = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(&command)
        .output()
        .await
        .map_err(|e| format!("Command failed: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

/// Tauri command: Show/hide window
#[tauri::command]
pub async fn toggle_window(window: Window) -> Result<(), String> {
    if window.is_visible().map_err(|e| e.to_string())? {
        window.hide().map_err(|e| e.to_string())?;
    } else {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Tauri command: Set window opacity
#[tauri::command]
pub async fn set_opacity(window: Window, opacity: f64) -> Result<(), String> {
    // Note: Wayland doesn't support window opacity via standard APIs
    // This would need Hyprland-specific implementation
    tracing::warn!("Window opacity not yet implemented for Wayland");
    Ok(())
}

/// Tauri command: Get recent problems
#[tauri::command]
pub async fn get_recent_problems() -> Result<Vec<ProblemNotification>, String> {
    // TODO: Implement problem history from KnowledgeBase
    Ok(vec![ProblemNotification {
        severity: "warning".to_string(),
        title: "Memory Pressure".to_string(),
        message: "Memory usage reached 85%".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    }])
}

/// Tauri command: Set autonomy level
#[tauri::command]
pub async fn set_autonomy_level(level: u8) -> Result<(), String> {
    if level > 100 {
        return Err("Autonomy level must be 0-100".to_string());
    }

    tracing::info!("Setting autonomy level to: {}", level);
    // TODO: Actually set the autonomy level in DecisionEngine
    Ok(())
}
