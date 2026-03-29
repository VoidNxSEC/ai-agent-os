//! Decision Engine - Intelligent Decision Making
//!
//! Decides what action to take for detected problems based on context and history.

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::knowledge_base::KnowledgeBase;
use crate::state_manager::StateManager;
use crate::{Decision, Problem, RemediationAction};

/// Intelligent decision making engine
pub struct DecisionEngine {
    /// State manager for context
    state: Arc<RwLock<StateManager>>,

    /// Knowledge base for historical decisions
    knowledge: Arc<RwLock<KnowledgeBase>>,

    /// Autonomy level (0=manual, 100=full autonomy)
    autonomy_level: u8,
}

impl DecisionEngine {
    /// Create new decision engine
    pub fn new(state: Arc<RwLock<StateManager>>, knowledge: Arc<RwLock<KnowledgeBase>>) -> Self {
        Self {
            state,
            knowledge,
            autonomy_level: 80, // Start with high autonomy
        }
    }

    /// Decide what to do about a problem
    pub async fn decide(&self, problem: &Problem) -> Result<Decision> {
        info!("🤔 Making decision for problem: {:?}", problem);

        // Check if we've seen this before
        let knowledge = self.knowledge.read().await;
        let historical_success = knowledge.get_success_rate(problem).await?;

        // Make decision based on problem type and history
        let decision = match problem {
            Problem::MemoryPressure { usage_percent } => {
                if *usage_percent > 95.0 {
                    // Critical - auto-fix immediately
                    Decision::AutoFix(RemediationAction::KillProcess {
                        pid: self.find_memory_hog().await?,
                        name: "memory-intensive-process".to_string(),
                    })
                } else if *usage_percent > 90.0 && historical_success > 0.8 {
                    // High and we're confident - auto-fix
                    Decision::AutoFix(RemediationAction::KillProcess {
                        pid: self.find_memory_hog().await?,
                        name: "high-memory-process".to_string(),
                    })
                } else {
                    // Moderate - just observe for now
                    Decision::Observe
                }
            }

            Problem::CpuOverload { load } => {
                if *load > 95.0 {
                    Decision::AutoFix(RemediationAction::ThrottleCpu)
                } else {
                    Decision::Observe
                }
            }

            Problem::ThermalIssue { temp_celsius } => {
                if *temp_celsius > 90.0 {
                    // Critical temperature - immediate action
                    Decision::AutoFix(RemediationAction::ThrottleCpu)
                } else if *temp_celsius > 85.0 {
                    Decision::NotifyUser(format!(
                        "High temperature: {:.1}°C - Consider throttling",
                        temp_celsius
                    ))
                } else {
                    Decision::Observe
                }
            }

            Problem::DiskFull {
                path,
                usage_percent,
            } => {
                if *usage_percent > 95.0 && historical_success > 0.9 {
                    Decision::AutoFix(RemediationAction::CleanDisk { path: path.clone() })
                } else if *usage_percent > 90.0 {
                    Decision::NotifyUser(format!(
                        "Disk {}% full at {} - cleanup recommended",
                        usage_percent, path
                    ))
                } else {
                    Decision::Observe
                }
            }

            Problem::ServiceDown { service } => {
                if self.is_auto_restartable(service) && historical_success > 0.85 {
                    Decision::AutoFix(RemediationAction::RestartService {
                        name: service.clone(),
                    })
                } else {
                    Decision::NotifyUser(format!(
                        "Service {} is down - manual intervention may be needed",
                        service
                    ))
                }
            }

            Problem::LogAnomaly { pattern } => {
                // Anomalies usually need human judgment
                Decision::NotifyUser(format!("Anomaly detected: {}", pattern))
            }
        };

        info!("✅ Decision made: {:?}", decision);
        Ok(decision)
    }

    /// Find the process using most memory
    async fn find_memory_hog(&self) -> Result<u32> {
        // TODO: Integrate with system-monitor to find actual memory hog
        // For now, return dummy PID
        Ok(12345)
    }

    /// Check if a service can be auto-restarted
    fn is_auto_restartable(&self, service: &str) -> bool {
        // Safe services that can be auto-restarted
        const SAFE_SERVICES: &[&str] = &["nginx", "docker", "postgresql", "redis"];

        SAFE_SERVICES.iter().any(|&safe| service.contains(safe))
    }

    /// Set autonomy level (0-100)
    pub fn set_autonomy(&mut self, level: u8) {
        self.autonomy_level = level.min(100);
        info!("🎚️ Autonomy level set to: {}", self.autonomy_level);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_restartability() {
        let state = Arc::new(RwLock::new(StateManager::new(Arc::new(RwLock::new(
            KnowledgeBase::new_mock(),
        )))));
        let knowledge = Arc::new(RwLock::new(KnowledgeBase::new_mock()));
        let engine = DecisionEngine::new(state, knowledge);

        assert!(engine.is_auto_restartable("nginx"));
        assert!(!engine.is_auto_restartable("sshd"));
    }
}
