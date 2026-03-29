//! State Manager - Context, Learning, and Memory Management
//!
//! Maintains the agent's understanding of the system state over time.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::knowledge_base::KnowledgeBase;
use crate::{AgentState, SystemAnalysis};

const MAX_HISTORY_SIZE: usize = 1000;
const STATE_RETENTION_HOURS: i64 = 24;

/// Manages system state, context, and learning
pub struct StateManager {
    /// Historical state snapshots
    history: VecDeque<StateSnapshot>,

    /// Current agent state
    current_state: AgentStateInternal,

    /// Knowledge base reference
    knowledge: Arc<RwLock<KnowledgeBase>>,

    /// Agent start time
    start_time: DateTime<Utc>,
}

impl StateManager {
    /// Create new state manager
    pub fn new(knowledge: Arc<RwLock<KnowledgeBase>>) -> Self {
        Self {
            history: VecDeque::with_capacity(MAX_HISTORY_SIZE),
            current_state: AgentStateInternal::default(),
            knowledge,
            start_time: Utc::now(),
        }
    }

    /// Record a state snapshot
    pub fn record_snapshot(&mut self, snapshot: StateSnapshot) {
        if self.history.len() >= MAX_HISTORY_SIZE {
            self.history.pop_front();
        }
        self.history.push_back(snapshot);
    }

    /// Get current agent state
    pub fn get_current_state(&self) -> AgentState {
        let uptime = (Utc::now() - self.start_time).num_seconds() as u64;

        AgentState {
            is_learning: self.current_state.is_learning,
            problems_detected: self.current_state.problems_detected,
            problems_fixed: self.current_state.problems_fixed,
            uptime_seconds: uptime,
            last_action: self.current_state.last_action.clone(),
        }
    }

    /// Analyze current system state
    pub async fn analyze_current_state(&self) -> Result<SystemAnalysis> {
        let mut problems = Vec::new();
        let mut recommendations = Vec::new();
        let mut anomalies = Vec::new();

        // Analyze recent history
        let recent = self.get_recent_snapshots(100);

        // Check for patterns
        if let Some(pattern) = self.detect_memory_leak(&recent) {
            problems.push(format!("Memory leak detected: {}", pattern));
            recommendations.push("Restart affected service".to_string());
        }

        if let Some(pattern) = self.detect_cpu_spike(&recent) {
            problems.push(format!("CPU spike pattern: {}", pattern));
            recommendations.push("Investigate high CPU processes".to_string());
        }

        // Calculate health score (0-100)
        let health_score = self.calculate_health_score(&recent);

        Ok(SystemAnalysis {
            health_score,
            problems,
            recommendations,
            anomalies,
        })
    }

    /// Update internal state
    pub fn update_state(&mut self, update: StateUpdate) {
        match update {
            StateUpdate::ProblemDetected => {
                self.current_state.problems_detected += 1;
            }
            StateUpdate::ProblemFixed => {
                self.current_state.problems_fixed += 1;
            }
            StateUpdate::ActionExecuted(action) => {
                self.current_state.last_action = Some(action);
            }
            StateUpdate::LearningStarted => {
                self.current_state.is_learning = true;
            }
            StateUpdate::LearningCompleted => {
                self.current_state.is_learning = false;
            }
        }
    }

    /// Clean up old state data
    pub fn cleanup_old_data(&mut self) -> Result<usize> {
        let cutoff = Utc::now() - chrono::Duration::hours(STATE_RETENTION_HOURS);
        let initial_len = self.history.len();

        self.history.retain(|snapshot| snapshot.timestamp > cutoff);

        let removed = initial_len - self.history.len();
        Ok(removed)
    }

    /// Get recent snapshots
    fn get_recent_snapshots(&self, count: usize) -> Vec<&StateSnapshot> {
        self.history.iter().rev().take(count).collect()
    }

    /// Detect memory leak pattern
    fn detect_memory_leak(&self, snapshots: &[&StateSnapshot]) -> Option<String> {
        if snapshots.len() < 10 {
            return None;
        }

        // Simple heuristic: memory steadily increasing
        let memory_values: Vec<f32> = snapshots.iter().map(|s| s.memory_percent).collect();

        let trend = self.calculate_trend(&memory_values);

        if trend > 0.5 && memory_values.last().unwrap_or(&0.0) > &80.0 {
            Some(format!("Memory increasing at {:.1}% per sample", trend))
        } else {
            None
        }
    }

    /// Detect CPU spike pattern
    fn detect_cpu_spike(&self, snapshots: &[&StateSnapshot]) -> Option<String> {
        if snapshots.len() < 5 {
            return None;
        }

        let cpu_values: Vec<f32> = snapshots.iter().map(|s| s.cpu_percent).collect();

        let avg = cpu_values.iter().sum::<f32>() / cpu_values.len() as f32;
        let max = cpu_values.iter().cloned().fold(0.0f32, f32::max);

        if max > 90.0 && (max - avg) > 30.0 {
            Some(format!("CPU spiked to {:.1}% (avg: {:.1}%)", max, avg))
        } else {
            None
        }
    }

    /// Calculate health score (0-100)
    fn calculate_health_score(&self, snapshots: &[&StateSnapshot]) -> f32 {
        if snapshots.is_empty() {
            return 100.0;
        }

        let latest = snapshots[0];
        let mut score: f32 = 100.0;

        // Penalize high resource usage
        if latest.cpu_percent > 80.0 {
            score -= 20.0;
        } else if latest.cpu_percent > 60.0 {
            score -= 10.0;
        }

        if latest.memory_percent > 85.0 {
            score -= 25.0;
        } else if latest.memory_percent > 70.0 {
            score -= 15.0;
        }

        if latest.temp_celsius > 80.0 {
            score -= 15.0;
        }

        score.max(0.0)
    }

    /// Calculate trend (simple linear regression slope)
    fn calculate_trend(&self, values: &[f32]) -> f32 {
        if values.len() < 2 {
            return 0.0;
        }

        let n = values.len() as f32;
        let x_sum: f32 = (0..values.len()).map(|i| i as f32).sum();
        let y_sum: f32 = values.iter().sum();
        let xy_sum: f32 = values.iter().enumerate().map(|(i, &y)| i as f32 * y).sum();
        let x2_sum: f32 = (0..values.len()).map(|i| (i as f32).powi(2)).sum();

        (n * xy_sum - x_sum * y_sum) / (n * x2_sum - x_sum.powi(2))
    }
}

/// Internal agent state
#[derive(Default)]
struct AgentStateInternal {
    is_learning: bool,
    problems_detected: usize,
    problems_fixed: usize,
    last_action: Option<String>,
}

/// State snapshot at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub timestamp: DateTime<Utc>,
    pub cpu_percent: f32,
    pub memory_percent: f32,
    pub temp_celsius: f32,
    pub disk_percent: f32,
    pub active_processes: usize,
}

/// State update event
pub enum StateUpdate {
    ProblemDetected,
    ProblemFixed,
    ActionExecuted(String),
    LearningStarted,
    LearningCompleted,
}
