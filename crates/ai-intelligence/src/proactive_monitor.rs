//! Proactive Monitor - Predictive Problem Detection
//!
//! Continuously monitors system state and predicts/detects problems before they become critical.

use anyhow::Result;
use std::cell::RefCell;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::anomaly_detector::AnomalyDetector;
use crate::{
    state_manager::{StateManager, StateSnapshot},
    Problem,
};

const MONITOR_INTERVAL_SECS: u64 = 5;
const MEMORY_THRESHOLD: f32 = 85.0;
const CPU_THRESHOLD: f32 = 90.0;
const TEMP_THRESHOLD: f32 = 80.0;
const DISK_THRESHOLD: f32 = 90.0;

/// Proactive problem detection monitor
pub struct ProactiveMonitor {
    /// State manager reference
    state: Arc<RwLock<StateManager>>,

    /// Anomaly detector
    anomaly_detector: Arc<RwLock<AnomalyDetector>>,

    /// Problem notification channel
    problem_tx: mpsc::UnboundedSender<Problem>,
    problem_rx: RefCell<Option<mpsc::UnboundedReceiver<Problem>>>,
}

impl ProactiveMonitor {
    /// Create new proactive monitor
    pub fn new(
        state: Arc<RwLock<StateManager>>,
        anomaly_detector: Arc<RwLock<AnomalyDetector>>,
    ) -> Self {
        let (problem_tx, problem_rx) = mpsc::unbounded_channel();

        Self {
            state,
            anomaly_detector,
            problem_tx,
            problem_rx: Some(problem_rx),
        }
    }

    /// Start monitoring task
    pub async fn start_monitoring(&self) {
        let state = self.state.clone();
        let anomaly_detector = self.anomaly_detector.clone();
        let problem_tx = self.problem_tx.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(MONITOR_INTERVAL_SECS)).await;

                if let Err(e) = Self::check_system_state(
                    state.clone(),
                    anomaly_detector.clone(),
                    problem_tx.clone(),
                )
                .await
                {
                    warn!("Monitor error: {}", e);
                }
            }
        });
    }

    /// Wait for next detected problem
    pub async fn detect_next_problem(&self) -> Option<Problem> {
        if let Some(rx) = self.problem_rx.borrow_mut().as_mut() {
            rx.recv().await
        } else {
            None
        }
    }

    /// Check current system state for problems
    async fn check_system_state(
        state: Arc<RwLock<StateManager>>,
        anomaly_detector: Arc<RwLock<AnomalyDetector>>,
        problem_tx: mpsc::UnboundedSender<Problem>,
    ) -> Result<()> {
        // Get current metrics (would integrate with system-monitor here)
        let snapshot = Self::capture_snapshot().await?;

        // Record in state manager
        {
            let mut state = state.write().await;
            state.record_snapshot(snapshot.clone());
        }

        // Check thresholds
        Self::check_thresholds(&snapshot, &problem_tx)?;

        // Check for anomalies
        let detector = anomaly_detector.read().await;
        if let Some(anomaly) = detector.detect_anomaly(&snapshot)? {
            info!("🔍 Anomaly detected: {}", anomaly);
            problem_tx.send(Problem::LogAnomaly { pattern: anomaly })?;
        }

        Ok(())
    }

    /// Check threshold-based problems
    fn check_thresholds(
        snapshot: &StateSnapshot,
        problem_tx: &mpsc::UnboundedSender<Problem>,
    ) -> Result<()> {
        // Memory pressure
        if snapshot.memory_percent > MEMORY_THRESHOLD {
            warn!("⚠️ Memory pressure: {:.1}%", snapshot.memory_percent);
            problem_tx.send(Problem::MemoryPressure {
                usage_percent: snapshot.memory_percent,
            })?;
        }

        // CPU overload
        if snapshot.cpu_percent > CPU_THRESHOLD {
            warn!("⚠️ CPU overload: {:.1}%", snapshot.cpu_percent);
            problem_tx.send(Problem::CpuOverload {
                load: snapshot.cpu_percent,
            })?;
        }

        // Thermal issue
        if snapshot.temp_celsius > TEMP_THRESHOLD {
            warn!("⚠️ High temperature: {:.1}°C", snapshot.temp_celsius);
            problem_tx.send(Problem::ThermalIssue {
                temp_celsius: snapshot.temp_celsius,
            })?;
        }

        // Disk full
        if snapshot.disk_percent > DISK_THRESHOLD {
            warn!("⚠️ Disk almost full: {:.1}%", snapshot.disk_percent);
            problem_tx.send(Problem::DiskFull {
                path: "/".to_string(),
                usage_percent: snapshot.disk_percent,
            })?;
        }

        Ok(())
    }

    /// Capture current system snapshot
    async fn capture_snapshot() -> Result<StateSnapshot> {
        // TODO: Integrate with actual system-monitor
        // For now, returning dummy data
        Ok(StateSnapshot {
            timestamp: chrono::Utc::now(),
            cpu_percent: 45.0,
            memory_percent: 55.0,
            temp_celsius: 65.0,
            disk_percent: 60.0,
            active_processes: 250,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_threshold_detection() {
        let (tx, mut rx) = mpsc::unbounded_channel();

        let snapshot = StateSnapshot {
            timestamp: chrono::Utc::now(),
            cpu_percent: 95.0,
            memory_percent: 90.0,
            temp_celsius: 85.0,
            disk_percent: 95.0,
            active_processes: 300,
        };

        ProactiveMonitor::check_thresholds(&snapshot, &tx).unwrap();

        // Should detect multiple problems
        let mut problems = Vec::new();
        while let Ok(problem) = rx.try_recv() {
            problems.push(problem);
        }

        assert!(problems.len() >= 3); // CPU, Memory, Temp, Disk
    }
}
