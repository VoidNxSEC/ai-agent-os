//! AI Intelligence Core - Autonomous System Agent
//!
//! This is the brain of the AI Agent OS - it learns, predicts, and acts autonomously.

pub mod anomaly_detector;
pub mod auto_remediation;
pub mod decision_engine;
pub mod knowledge_base;
pub mod proactive_monitor;
pub mod state_manager;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use hyprland_ipc::HyprlandClient;
use log_collector::LogCollector;
use system_monitor::SystemMonitor;

pub use anomaly_detector::AnomalyDetector;
pub use auto_remediation::AutoRemediation;
pub use decision_engine::DecisionEngine;
pub use knowledge_base::KnowledgeBase;
pub use proactive_monitor::ProactiveMonitor;
pub use state_manager::StateManager;

/// The autonomous AI agent core
pub struct IntelligentAgent {
    /// Shared system state and learning
    state: Arc<RwLock<StateManager>>,

    /// Proactive problem detection
    monitor: Arc<ProactiveMonitor>,

    /// Automatic problem fixing
    remediation: Arc<AutoRemediation>,

    /// Decision making engine
    decision: Arc<DecisionEngine>,

    /// Historical knowledge
    knowledge: Arc<RwLock<KnowledgeBase>>,

    /// Anomaly detection ML
    anomaly_detector: Arc<RwLock<AnomalyDetector>>,
}

impl IntelligentAgent {
    /// Create new intelligent agent
    pub async fn new() -> Result<Self> {
        info!("🧠 Initializing Intelligent Agent...");

        let knowledge = Arc::new(RwLock::new(KnowledgeBase::new().await?));
        let state = Arc::new(RwLock::new(StateManager::new(knowledge.clone())));
        let anomaly_detector = Arc::new(RwLock::new(AnomalyDetector::new()));

        let monitor = Arc::new(ProactiveMonitor::new(
            state.clone(),
            anomaly_detector.clone(),
        ));

        let remediation = Arc::new(AutoRemediation::new(knowledge.clone()));
        let decision = Arc::new(DecisionEngine::new(state.clone(), knowledge.clone()));

        Ok(Self {
            state,
            monitor,
            remediation,
            decision,
            knowledge,
            anomaly_detector,
        })
    }

    /// Start the autonomous intelligence loop
    pub async fn run(&self) -> Result<()> {
        info!("🚀 Starting Autonomous Intelligence Loop");

        loop {
            tokio::select! {
                // Proactive monitoring (continuous)
                problem = self.monitor.detect_next_problem() => {
                    if let Some(problem) = problem {
                        self.handle_problem(problem).await?;
                    }
                }

                // Periodic learning (every 5 minutes)
                _ = tokio::time::sleep(std::time::Duration::from_secs(300)) => {
                    self.learn_from_history().await?;
                }

                // State cleanup (every hour)
                _ = tokio::time::sleep(std::time::Duration::from_secs(3600)) => {
                    self.cleanup_old_state().await?;
                }
            }
        }
    }

    /// Handle detected problem autonomously
    async fn handle_problem(&self, problem: Problem) -> Result<()> {
        info!("⚠️ Problem detected: {:?}", problem);

        // Ask decision engine what to do
        let decision = self.decision.decide(&problem).await?;

        match decision {
            Decision::AutoFix(action) => {
                info!("🔧 Auto-fixing: {:?}", action);

                match self.remediation.execute(action).await {
                    Ok(result) => {
                        info!("✅ Problem fixed: {:?}", result);
                        self.knowledge
                            .write()
                            .await
                            .record_success(problem, result)
                            .await?;
                    }
                    Err(e) => {
                        error!("❌ Auto-fix failed: {}", e);
                        self.knowledge
                            .write()
                            .await
                            .record_failure(problem, e.to_string())
                            .await?;
                    }
                }
            }

            Decision::NotifyUser(reason) => {
                warn!("👤 User notification required: {}", reason);
                // Future: Send notification via GUI
            }

            Decision::Observe => {
                info!("👁️ Observing problem, will act if worsens");
            }
        }

        Ok(())
    }

    /// Learn from historical data
    async fn learn_from_history(&self) -> Result<()> {
        info!("📚 Learning from historical data...");

        let knowledge = self.knowledge.read().await;
        let patterns = knowledge.extract_patterns().await?;

        let mut detector = self.anomaly_detector.write().await;
        detector.update_model(&patterns)?;

        info!(
            "✅ Learning complete: {} patterns identified",
            patterns.len()
        );
        Ok(())
    }

    /// Cleanup old state data
    async fn cleanup_old_state(&self) -> Result<()> {
        info!("🧹 Cleaning up old state...");

        let mut state = self.state.write().await;
        let cleaned = state.cleanup_old_data()?;

        info!("✅ Cleanup complete: {} entries removed", cleaned);
        Ok(())
    }

    /// Get current agent state
    pub async fn get_state(&self) -> AgentState {
        let state = self.state.read().await;
        state.get_current_state()
    }

    /// Force analysis of current system state
    pub async fn analyze_now(&self) -> Result<SystemAnalysis> {
        info!("🔍 Performing forced system analysis...");

        let state = self.state.read().await;
        let analysis = state.analyze_current_state().await?;

        Ok(analysis)
    }
}

/// Problem detected by proactive monitor
#[derive(Debug, Clone, Serialize)]
pub enum Problem {
    MemoryPressure { usage_percent: f32 },
    CpuOverload { load: f32 },
    DiskFull { path: String, usage_percent: f32 },
    ServiceDown { service: String },
    LogAnomaly { pattern: String },
    ThermalIssue { temp_celsius: f32 },
}

/// Decision made by decision engine
#[derive(Debug, Clone)]
pub enum Decision {
    AutoFix(RemediationAction),
    NotifyUser(String),
    Observe,
}

/// Action to fix a problem
#[derive(Debug, Clone)]
pub enum RemediationAction {
    KillProcess { pid: u32, name: String },
    RestartService { name: String },
    CleanDisk { path: String },
    ThrottleCpu,
    RestartApplication { name: String },
}

/// Result of a remediation action
#[derive(Debug, Clone)]
pub struct RemediationResult {
    pub success: bool,
    pub message: String,
    pub metrics_before: String,
    pub metrics_after: String,
}

/// Current state of the agent
#[derive(Debug, Clone, serde::Serialize)]
pub struct AgentState {
    pub is_learning: bool,
    pub problems_detected: usize,
    pub problems_fixed: usize,
    pub uptime_seconds: u64,
    pub last_action: Option<String>,
}

/// System analysis result
#[derive(Debug, Clone, serde::Serialize)]
pub struct SystemAnalysis {
    pub health_score: f32,
    pub problems: Vec<String>,
    pub recommendations: Vec<String>,
    pub anomalies: Vec<String>,
}
