//! Knowledge Base - Historical Learning and Pattern Storage
//!
//! Stores and retrieves historical problem-solution pairs for learning.

use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{Problem, RemediationResult};

/// Knowledge base for storing historical data
pub struct KnowledgeBase {
    /// SQLite connection
    conn: Mutex<Connection>,
}

impl KnowledgeBase {
    /// Create new knowledge base
    pub async fn new() -> Result<Self> {
        let conn = Mutex::new(Connection::open("agent-knowledge.db")?);

        // Create tables
        conn.execute(
            "CREATE TABLE IF NOT EXISTS actions (
                id INTEGER PRIMARY KEY,
                timestamp TEXT NOT NULL,
                problem_type TEXT NOT NULL,
                problem_data TEXT NOT NULL,
                action_type TEXT NOT NULL,
                action_data TEXT NOT NULL,
                success INTEGER NOT NULL,
                result_message TEXT,
                metrics_before TEXT,
                metrics_after TEXT
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS patterns (
                id INTEGER PRIMARY KEY,
                pattern_type TEXT NOT NULL,
                pattern_data TEXT NOT NULL,
                frequency INTEGER NOT NULL,
                last_seen TEXT NOT NULL
            )",
            [],
        )?;

        Ok(Self { conn })
    }

    /// Record a successful action
    pub async fn record_success(&self, problem: Problem, result: RemediationResult) -> Result<()> {
        let timestamp = Utc::now().to_rfc3339();
        let problem_type = format!("{:?}", problem)
            .split('(')
            .next()
            .unwrap_or("Unknown")
            .to_string();
        let problem_data = serde_json::to_string(&problem)?;

        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO actions (
                timestamp, problem_type, problem_data,
                action_type, action_data, success,
                result_message, metrics_before, metrics_after
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                timestamp,
                problem_type,
                problem_data,
                "AutoFix",
                "",
                1,
                result.message,
                result.metrics_before,
                result.metrics_after,
            ],
        )?;

        Ok(())
    }

    /// Record a failed action
    pub async fn record_failure(&self, problem: Problem, error: String) -> Result<()> {
        let timestamp = Utc::now().to_rfc3339();
        let problem_type = format!("{:?}", problem)
            .split('(')
            .next()
            .unwrap_or("Unknown")
            .to_string();
        let problem_data = serde_json::to_string(&problem)?;

        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO actions (
                timestamp, problem_type, problem_data,
                action_type, action_data, success,
                result_message, metrics_before, metrics_after
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                timestamp,
                problem_type,
                problem_data,
                "AutoFix",
                "",
                0,
                error,
                "",
                "",
            ],
        )?;

        Ok(())
    }

    /// Get success rate for a problem type
    pub async fn get_success_rate(&self, problem: &Problem) -> Result<f32> {
        let problem_type = format!("{:?}", problem)
            .split('(')
            .next()
            .unwrap_or("Unknown")
            .to_string();

        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT 
                SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END) as successes,
                COUNT(*) as total
            FROM actions
            WHERE problem_type = ?1",
        )?;

        let (successes, total): (i32, i32) = stmt
            .query_row(params![problem_type], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap_or((0, 0));

        if total == 0 {
            Ok(0.5) // No history, assume 50% success
        } else {
            Ok(successes as f32 / total as f32)
        }
    }

    /// Extract patterns from historical data
    pub async fn extract_patterns(&self) -> Result<Vec<Pattern>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT problem_type, COUNT(*) as frequency
            FROM actions
            WHERE timestamp > datetime('now', '-30 days')
            GROUP BY problem_type
            ORDER BY frequency DESC
            LIMIT 10",
        )?;

        let patterns = stmt
            .query_map([], |row| {
                Ok(Pattern {
                    pattern_type: row.get(0)?,
                    frequency: row.get(1)?,
                    last_seen: Utc::now(),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(patterns)
    }

    /// Create a mock knowledge base for testing
    #[cfg(test)]
    pub fn new_mock() -> Self {
        let conn = Connection::open_in_memory().unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS actions (
                id INTEGER PRIMARY KEY,
                timestamp TEXT NOT NULL,
                problem_type TEXT NOT NULL,
                problem_data TEXT NOT NULL,
                action_type TEXT NOT NULL,
                action_data TEXT NOT NULL,
                success INTEGER NOT NULL,
                result_message TEXT,
                metrics_before TEXT,
                metrics_after TEXT
            )",
            [],
        )
        .unwrap();

        Self { conn }
    }
}

/// Detected pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub pattern_type: String,
    pub frequency: i32,
    pub last_seen: DateTime<Utc>,
}
