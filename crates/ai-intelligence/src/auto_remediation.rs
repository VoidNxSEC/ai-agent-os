//! Auto Remediation - Autonomous Problem Fixing
//!
//! Executes automated fixes for detected problems without user intervention.

use anyhow::{anyhow, Result};
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::knowledge_base::KnowledgeBase;
use crate::{RemediationAction, RemediationResult};

/// Autonomous problem remediation system
pub struct AutoRemediation {
    /// Knowledge base for learning from fixes
    knowledge: Arc<RwLock<KnowledgeBase>>,

    /// Safety mode (true = ask before critical actions)
    safe_mode: bool,
}

impl AutoRemediation {
    /// Create new auto-remediation system
    pub fn new(knowledge: Arc<RwLock<KnowledgeBase>>) -> Self {
        Self {
            knowledge,
            safe_mode: true, // Start in safe mode
        }
    }

    /// Execute a remediation action
    pub async fn execute(&self, action: RemediationAction) -> Result<RemediationResult> {
        info!("🔧 Executing remediation: {:?}", action);

        let metrics_before = self.capture_metrics().await?;

        let result = match action {
            RemediationAction::KillProcess { pid, ref name } => {
                self.kill_process(pid, name).await?
            }

            RemediationAction::RestartService { ref name } => self.restart_service(name).await?,

            RemediationAction::CleanDisk { ref path } => self.clean_disk(path).await?,

            RemediationAction::ThrottleCpu => self.throttle_cpu().await?,

            RemediationAction::RestartApplication { ref name } => {
                self.restart_application(name).await?
            }
        };

        let metrics_after = self.capture_metrics().await?;

        Ok(RemediationResult {
            success: result.success,
            message: result.message,
            metrics_before,
            metrics_after,
        })
    }

    /// Kill a process
    async fn kill_process(&self, pid: u32, name: &str) -> Result<ActionResult> {
        info!("💀 Killing process: {} (PID: {})", name, pid);

        if self.safe_mode {
            // In safe mode, only kill non-critical processes
            if self.is_critical_process(name) {
                return Ok(ActionResult {
                    success: false,
                    message: format!("Process {} is critical, skipping", name),
                });
            }
        }

        let output = Command::new("kill")
            .arg("-9")
            .arg(pid.to_string())
            .output()
            .await?;

        if output.status.success() {
            info!("✅ Process {} killed successfully", name);
            Ok(ActionResult {
                success: true,
                message: format!("Killed process {} (PID: {})", name, pid),
            })
        } else {
            let err = String::from_utf8_lossy(&output.stderr);
            error!("❌ Failed to kill process: {}", err);
            Ok(ActionResult {
                success: false,
                message: format!("Failed to kill process: {}", err),
            })
        }
    }

    /// Restart a systemd service
    async fn restart_service(&self, name: &str) -> Result<ActionResult> {
        info!("🔄 Restarting service: {}", name);

        let output = Command::new("systemctl")
            .arg("restart")
            .arg(name)
            .output()
            .await?;

        if output.status.success() {
            info!("✅ Service {} restarted", name);
            Ok(ActionResult {
                success: true,
                message: format!("Service {} restarted successfully", name),
            })
        } else {
            let err = String::from_utf8_lossy(&output.stderr);
            error!("❌ Failed to restart service: {}", err);
            Ok(ActionResult {
                success: false,
                message: format!("Failed to restart: {}", err),
            })
        }
    }

    /// Clean disk space
    async fn clean_disk(&self, path: &str) -> Result<ActionResult> {
        info!("🧹 Cleaning disk: {}", path);

        let mut cleaned_mb = 0;

        // Clean journald logs
        let output = Command::new("journalctl")
            .arg("--vacuum-size=100M")
            .output()
            .await?;

        if output.status.success() {
            cleaned_mb += 100;
        }

        // Clean package cache (cautiously)
        if path == "/" {
            let output = Command::new("nix-collect-garbage")
                .arg("-d")
                .output()
                .await?;

            if output.status.success() {
                cleaned_mb += 500; // Estimate
            }
        }

        info!("✅ Cleaned approximately {}MB", cleaned_mb);
        Ok(ActionResult {
            success: true,
            message: format!("Cleaned ~{}MB of disk space", cleaned_mb),
        })
    }

    /// Throttle CPU (reduce power/performance)
    async fn throttle_cpu(&self) -> Result<ActionResult> {
        info!("🐌 Throttling CPU to reduce heat/load");

        // Set CPU governor to powersave
        let output = Command::new("cpupower")
            .arg("frequency-set")
            .arg("-g")
            .arg("powersave")
            .output()
            .await;

        match output {
            Ok(out) if out.status.success() => {
                info!("✅ CPU throttled to powersave mode");
                Ok(ActionResult {
                    success: true,
                    message: "CPU set to powersave mode".to_string(),
                })
            }
            _ => {
                warn!("⚠️ Could not throttle CPU (cpupower not available?)");
                Ok(ActionResult {
                    success: false,
                    message: "CPU throttling unavailable".to_string(),
                })
            }
        }
    }

    /// Restart an application
    async fn restart_application(&self, name: &str) -> Result<ActionResult> {
        info!("🔄 Restarting application: {}", name);

        // Find and kill the process
        let output = Command::new("pkill").arg(name).output().await?;

        if output.status.success() {
            // Wait a bit for graceful shutdown
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;

            // Try to restart (if it's a known app)
            // This would need to be more sophisticated in practice
            info!("✅ Application {} terminated (restart may be manual)", name);
            Ok(ActionResult {
                success: true,
                message: format!("Application {} terminated", name),
            })
        } else {
            Ok(ActionResult {
                success: false,
                message: format!("Could not find application {}", name),
            })
        }
    }

    /// Check if a process is critical
    fn is_critical_process(&self, name: &str) -> bool {
        const CRITICAL: &[&str] = &[
            "systemd",
            "init",
            "sshd",
            "dbus-daemon",
            "NetworkManager",
            "Xorg",
            "hyprland",
        ];

        CRITICAL.iter().any(|&critical| name.contains(critical))
    }

    /// Capture current system metrics
    async fn capture_metrics(&self) -> Result<String> {
        // TODO: Integrate with system-monitor
        Ok("cpu=45%,mem=55%,temp=65C".to_string())
    }
}

/// Result of a remediation action
struct ActionResult {
    success: bool,
    message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_critical_process_detection() {
        let knowledge = Arc::new(RwLock::new(KnowledgeBase::new_mock()));
        let remediation = AutoRemediation::new(knowledge);

        assert!(remediation.is_critical_process("systemd"));
        assert!(remediation.is_critical_process("hyprland"));
        assert!(!remediation.is_critical_process("firefox"));
    }
}
