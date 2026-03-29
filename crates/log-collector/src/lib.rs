//! Log collector module for systemd journal integration
//!
//! Provides streaming access to system logs via journald, with filtering
//! and real-time monitoring capabilities.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use systemd::journal::{Journal, JournalFiles, JournalRecord, JournalSeek};

/// Log entry from journald
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: u64,
    pub priority: LogPriority,
    pub message: String,
    pub unit: Option<String>,
    pub pid: Option<u32>,
    pub hostname: Option<String>,
    pub syslog_identifier: Option<String>,
    pub fields: HashMap<String, String>,
}

/// Log priority levels (from syslog)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogPriority {
    Emergency = 0,
    Alert = 1,
    Critical = 2,
    Error = 3,
    Warning = 4,
    Notice = 5,
    Info = 6,
    Debug = 7,
}

impl From<i32> for LogPriority {
    fn from(priority: i32) -> Self {
        match priority {
            0 => LogPriority::Emergency,
            1 => LogPriority::Alert,
            2 => LogPriority::Critical,
            3 => LogPriority::Error,
            4 => LogPriority::Warning,
            5 => LogPriority::Notice,
            6 => LogPriority::Info,
            _ => LogPriority::Debug,
        }
    }
}

/// Log filter criteria
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LogFilter {
    pub unit: Option<String>,
    pub priority: Option<LogPriority>,
    pub since: Option<u64>,
    pub until: Option<u64>,
    pub identifier: Option<String>,
}

/// Log collector for systemd journal
pub struct LogCollector {
    journal: Journal,
}

impl LogCollector {
    /// Create a new log collector
    pub fn new() -> Result<Self> {
        let journal = Journal::open(JournalFiles::All, false, true)
            .context("Failed to open systemd journal")?;

        Ok(Self { journal })
    }

    /// Seek to the end of the journal (for tailing)
    pub fn seek_tail(&mut self) -> Result<()> {
        self.journal
            .seek(JournalSeek::Tail)
            .context("Failed to seek to end of journal")?;
        Ok(())
    }

    /// Seek to the beginning of the journal
    pub fn seek_head(&mut self) -> Result<()> {
        self.journal
            .seek(JournalSeek::Head)
            .context("Failed to seek to beginning of journal")?;
        Ok(())
    }

    /// Apply filter to journal
    pub fn apply_filter(&mut self, filter: &LogFilter) -> Result<()> {
        if let Some(ref unit) = filter.unit {
            self.journal
                .match_add("_SYSTEMD_UNIT", unit.as_str())
                .context("Failed to add unit filter")?;
        }

        if let Some(ref identifier) = filter.identifier {
            self.journal
                .match_add("SYSLOG_IDENTIFIER", identifier.as_str())
                .context("Failed to add identifier filter")?;
        }

        if let Some(priority) = filter.priority {
            let priority_str = (priority as i32).to_string();
            self.journal
                .match_add("PRIORITY", priority_str.as_str())
                .context("Failed to add priority filter")?;
        }

        Ok(())
    }

    /// Get next log entry
    pub fn next_entry(&mut self) -> Result<Option<LogEntry>> {
        match self.journal.next_entry() {
            Ok(Some(record)) => Ok(Some(self.record_to_entry(record)?)),
            Ok(None) => Ok(None),
            Err(e) => Err(anyhow::anyhow!("Failed to read journal entry: {}", e)),
        }
    }

    /// Wait for new entries (blocking)
    pub fn wait(&mut self, timeout_micros: Option<u64>) -> Result<bool> {
        use std::time::Duration;

        let timeout = timeout_micros.map(|micros| Duration::from_micros(micros));
        let result = self
            .journal
            .wait(timeout)
            .context("Failed to wait for journal entries")?;

        Ok(matches!(
            result,
            systemd::journal::JournalWaitResult::Append
        ))
    }

    /// Stream entries continuously
    pub async fn stream_entries(&mut self) -> Result<LogEntryStream> {
        self.seek_tail()?;
        Ok(LogEntryStream { collector: self })
    }

    /// Get recent entries
    pub fn get_recent_entries(&mut self, count: usize) -> Result<Vec<LogEntry>> {
        self.seek_tail()?;

        let mut entries = Vec::new();
        for _ in 0..count {
            if let Some(entry) = self.journal.previous_entry()? {
                entries.push(self.record_to_entry(entry)?);
            } else {
                break;
            }
        }

        entries.reverse();
        Ok(entries)
    }

    /// Get entries for a specific unit
    pub fn get_unit_logs(&mut self, unit: &str, limit: usize) -> Result<Vec<LogEntry>> {
        let filter = LogFilter {
            unit: Some(unit.to_string()),
            ..Default::default()
        };

        self.apply_filter(&filter)?;
        self.seek_tail()?;

        let mut entries = Vec::new();
        for _ in 0..limit {
            if let Some(entry) = self.journal.previous_entry()? {
                entries.push(self.record_to_entry(entry)?);
            } else {
                break;
            }
        }

        entries.reverse();
        Ok(entries)
    }

    /// Convert JournalRecord to LogEntry
    fn record_to_entry(&self, record: JournalRecord) -> Result<LogEntry> {
        let mut fields = HashMap::new();

        // Extract all fields from the record
        for (key, value) in record.iter() {
            fields.insert(key.to_string(), value.to_string());
        }

        // Extract common fields
        let message = fields
            .get("MESSAGE")
            .cloned()
            .unwrap_or_else(|| "<no message>".to_string());

        let priority = fields
            .get("PRIORITY")
            .and_then(|p| p.parse::<i32>().ok())
            .map(LogPriority::from)
            .unwrap_or(LogPriority::Info);

        let unit = fields.get("_SYSTEMD_UNIT").cloned();

        let pid = fields.get("_PID").and_then(|p| p.parse::<u32>().ok());

        let hostname = fields.get("_HOSTNAME").cloned();

        let syslog_identifier = fields.get("SYSLOG_IDENTIFIER").cloned();

        let timestamp = fields
            .get("__REALTIME_TIMESTAMP")
            .and_then(|t| t.parse::<u64>().ok())
            .unwrap_or(0);

        Ok(LogEntry {
            timestamp,
            priority,
            message,
            unit,
            pid,
            hostname,
            syslog_identifier,
            fields,
        })
    }

    /// Check for critical errors in recent logs
    pub fn has_critical_errors(&mut self, lookback_minutes: u64) -> Result<bool> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let since = now - (lookback_minutes * 60);

        let filter = LogFilter {
            priority: Some(LogPriority::Critical),
            since: Some(since),
            ..Default::default()
        };

        self.apply_filter(&filter)?;
        self.seek_head()?;

        Ok(self.next_entry()?.is_some())
    }
}

impl Default for LogCollector {
    fn default() -> Self {
        Self::new().expect("Failed to create log collector")
    }
}

/// Stream of log entries
pub struct LogEntryStream<'a> {
    collector: &'a mut LogCollector,
}

impl<'a> LogEntryStream<'a> {
    /// Get next entry from stream
    pub fn next(&mut self) -> Result<Option<LogEntry>> {
        // Wait for new entries with 1 second timeout
        if self.collector.wait(Some(1_000_000))? {
            self.collector.next_entry()
        } else {
            Ok(None)
        }
    }
}

/// Helper to format log entries
impl LogEntry {
    /// Format as human-readable string
    pub fn format(&self) -> String {
        let priority_str = match self.priority {
            LogPriority::Emergency => "EMERG",
            LogPriority::Alert => "ALERT",
            LogPriority::Critical => "CRIT",
            LogPriority::Error => "ERROR",
            LogPriority::Warning => "WARN",
            LogPriority::Notice => "NOTICE",
            LogPriority::Info => "INFO",
            LogPriority::Debug => "DEBUG",
        };

        let unit_str = self
            .unit
            .as_ref()
            .map(|u| format!("[{}]", u))
            .unwrap_or_default();

        format!(
            "{} {} {} {}",
            self.timestamp, priority_str, unit_str, self.message
        )
    }

    /// Check if this is an error or higher priority
    pub fn is_error(&self) -> bool {
        (self.priority as i32) <= (LogPriority::Error as i32)
    }

    /// Check if this is a warning or higher priority
    pub fn is_warning(&self) -> bool {
        (self.priority as i32) <= (LogPriority::Warning as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_collector_creation() {
        let result = LogCollector::new();
        assert!(result.is_ok());
    }

    #[test]
    fn test_log_priority_conversion() {
        assert_eq!(LogPriority::from(0), LogPriority::Emergency);
        assert_eq!(LogPriority::from(3), LogPriority::Error);
        assert_eq!(LogPriority::from(6), LogPriority::Info);
    }
}
