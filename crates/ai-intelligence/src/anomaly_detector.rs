//! Anomaly Detector - ML-based Pattern Recognition
//!
//! Detects unusual patterns in system behavior using statistical analysis.

use anyhow::Result;
use statrs::statistics::{Data, Distribution, Max, Min, Statistics};
use std::collections::VecDeque;

use crate::knowledge_base::Pattern;
use crate::state_manager::StateSnapshot;

const WINDOW_SIZE: usize = 50;
const ANOMALY_THRESHOLD: f32 = 3.0; // Standard deviations

/// ML-based anomaly detection
pub struct AnomalyDetector {
    /// Historical data windows
    cpu_window: VecDeque<f32>,
    memory_window: VecDeque<f32>,
    temp_window: VecDeque<f32>,

    /// Learned baselines
    cpu_baseline: Option<Baseline>,
    memory_baseline: Option<Baseline>,
    temp_baseline: Option<Baseline>,
}

impl AnomalyDetector {
    /// Create new anomaly detector
    pub fn new() -> Self {
        Self {
            cpu_window: VecDeque::with_capacity(WINDOW_SIZE),
            memory_window: VecDeque::with_capacity(WINDOW_SIZE),
            temp_window: VecDeque::with_capacity(WINDOW_SIZE),
            cpu_baseline: None,
            memory_baseline: None,
            temp_baseline: None,
        }
    }

    /// Detect anomalies in current snapshot
    pub fn detect_anomaly(&self, snapshot: &StateSnapshot) -> Result<Option<String>> {
        let mut anomalies = Vec::new();

        // Check CPU anomaly
        if let Some(baseline) = &self.cpu_baseline {
            if Self::is_anomaly(snapshot.cpu_percent, baseline) {
                anomalies.push(format!(
                    "CPU anomaly: {:.1}% (baseline: {:.1}±{:.1})",
                    snapshot.cpu_percent, baseline.mean, baseline.stddev
                ));
            }
        }

        // Check memory anomaly
        if let Some(baseline) = &self.memory_baseline {
            if Self::is_anomaly(snapshot.memory_percent, baseline) {
                anomalies.push(format!(
                    "Memory anomaly: {:.1}% (baseline: {:.1}±{:.1})",
                    snapshot.memory_percent, baseline.mean, baseline.stddev
                ));
            }
        }

        // Check temperature anomaly
        if let Some(baseline) = &self.temp_baseline {
            if Self::is_anomaly(snapshot.temp_celsius, baseline) {
                anomalies.push(format!(
                    "Temperature anomaly: {:.1}°C (baseline: {:.1}±{:.1})",
                    snapshot.temp_celsius, baseline.mean, baseline.stddev
                ));
            }
        }

        if anomalies.is_empty() {
            Ok(None)
        } else {
            Ok(Some(anomalies.join("; ")))
        }
    }

    /// Update model with new patterns
    pub fn update_model(&mut self, patterns: &[Pattern]) -> Result<()> {
        // This is a simplified version - in a real implementation,
        // we would use more sophisticated ML models

        // For now, just recalculate baselines from windows
        if self.cpu_window.len() >= 10 {
            self.cpu_baseline = Some(Self::calculate_baseline(&self.cpu_window));
        }

        if self.memory_window.len() >= 10 {
            self.memory_baseline = Some(Self::calculate_baseline(&self.memory_window));
        }

        if self.temp_window.len() >= 10 {
            self.temp_baseline = Some(Self::calculate_baseline(&self.temp_window));
        }

        Ok(())
    }

    /// Add snapshot to training data
    pub fn add_snapshot(&mut self, snapshot: &StateSnapshot) {
        // Add to windows
        if self.cpu_window.len() >= WINDOW_SIZE {
            self.cpu_window.pop_front();
        }
        self.cpu_window.push_back(snapshot.cpu_percent);

        if self.memory_window.len() >= WINDOW_SIZE {
            self.memory_window.pop_front();
        }
        self.memory_window.push_back(snapshot.memory_percent);

        if self.temp_window.len() >= WINDOW_SIZE {
            self.temp_window.pop_front();
        }
        self.temp_window.push_back(snapshot.temp_celsius);
    }

    /// Check if value is an anomaly
    fn is_anomaly(value: f32, baseline: &Baseline) -> bool {
        let z_score = (value - baseline.mean) / baseline.stddev;
        z_score.abs() > ANOMALY_THRESHOLD
    }

    /// Calculate baseline statistics
    fn calculate_baseline(data: &VecDeque<f32>) -> Baseline {
        let values: Vec<f64> = data.iter().map(|&v| v as f64).collect();
        let data = Data::new(values);

        Baseline {
            mean: data.mean().unwrap_or(0.0) as f32,
            stddev: data.std_dev().unwrap_or(1.0) as f32,
            min: data.min() as f32,
            max: data.max() as f32,
        }
    }
}

/// Statistical baseline for a metric
#[derive(Debug, Clone)]
struct Baseline {
    mean: f32,
    stddev: f32,
    min: f32,
    max: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_anomaly_detection() {
        let mut detector = AnomalyDetector::new();

        // Train with normal data
        for i in 0..50 {
            detector.add_snapshot(&StateSnapshot {
                timestamp: Utc::now(),
                cpu_percent: 40.0 + (i as f32 % 10.0),
                memory_percent: 50.0,
                temp_celsius: 65.0,
                disk_percent: 60.0,
                active_processes: 250,
            });
        }

        // Update model
        detector.update_model(&[]).unwrap();

        // Test normal value
        let normal = StateSnapshot {
            timestamp: Utc::now(),
            cpu_percent: 42.0,
            memory_percent: 52.0,
            temp_celsius: 66.0,
            disk_percent: 60.0,
            active_processes: 250,
        };

        assert!(detector.detect_anomaly(&normal).unwrap().is_none());

        // Test anomalous value
        let anomaly = StateSnapshot {
            timestamp: Utc::now(),
            cpu_percent: 95.0, // Way above baseline
            memory_percent: 52.0,
            temp_celsius: 66.0,
            disk_percent: 60.0,
            active_processes: 250,
        };

        let result = detector.detect_anomaly(&anomaly).unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().contains("CPU anomaly"));
    }
}
