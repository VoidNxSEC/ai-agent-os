//! System monitoring module for collecting CPU, memory, disk, and thermal metrics
//!
//! Provides real-time system metrics with minimal overhead, optimized for
//! continuous monitoring scenarios.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use sysinfo::{Components, Disks, Networks, System};

/// System metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub timestamp: u64,
    pub cpu: CpuMetrics,
    pub memory: MemoryMetrics,
    pub disk: DiskMetrics,
    pub thermal: ThermalMetrics,
    pub network: NetworkMetrics,
}

/// CPU metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuMetrics {
    pub usage_percent: f32,
    pub cores: Vec<CoreMetrics>,
    pub load_average_1m: f32,
    pub load_average_5m: f32,
    pub load_average_15m: f32,
    pub frequency_mhz: u64,
}

/// Per-core CPU metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreMetrics {
    pub id: usize,
    pub usage_percent: f32,
    pub frequency_mhz: u64,
}

/// Memory metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetrics {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub usage_percent: f32,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
    pub swap_usage_percent: f32,
}

/// Disk metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskMetrics {
    pub disks: Vec<DiskInfo>,
    pub total_read_bytes: u64,
    pub total_write_bytes: u64,
}

/// Per-disk information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub usage_percent: f32,
    pub filesystem: String,
}

/// Thermal metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalMetrics {
    pub sensors: Vec<ThermalSensor>,
    pub max_temp_celsius: f32,
    pub avg_temp_celsius: f32,
}

/// Thermal sensor reading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalSensor {
    pub name: String,
    pub label: String,
    pub temperature_celsius: f32,
    pub critical_celsius: Option<f32>,
}

/// Network metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetrics {
    pub interfaces: Vec<NetworkInterface>,
    pub total_rx_bytes: u64,
    pub total_tx_bytes: u64,
}

/// Network interface metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_packets: u64,
    pub tx_packets: u64,
}

/// System monitor
pub struct SystemMonitor {
    sys: System,
    components: Components,
    disks: Disks,
    networks: Networks,
}

impl SystemMonitor {
    /// Create a new system monitor
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();

        Self {
            sys,
            components: Components::new_with_refreshed_list(),
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
        }
    }

    /// Refresh all metrics
    pub fn refresh(&mut self) {
        self.sys.refresh_all();
        self.components.refresh();
        self.disks.refresh();
        self.networks.refresh();
    }

    /// Collect current system metrics snapshot
    pub fn collect(&mut self) -> Result<SystemMetrics> {
        self.refresh();

        Ok(SystemMetrics {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            cpu: self.collect_cpu_metrics()?,
            memory: self.collect_memory_metrics()?,
            disk: self.collect_disk_metrics()?,
            thermal: self.collect_thermal_metrics()?,
            network: self.collect_network_metrics()?,
        })
    }

    fn collect_cpu_metrics(&self) -> Result<CpuMetrics> {
        let load_avg = System::load_average();
        let global_cpu = self.sys.global_cpu_usage();

        let cores = self
            .sys
            .cpus()
            .iter()
            .enumerate()
            .map(|(id, cpu)| CoreMetrics {
                id,
                usage_percent: cpu.cpu_usage(),
                frequency_mhz: cpu.frequency(),
            })
            .collect();

        Ok(CpuMetrics {
            usage_percent: global_cpu,
            cores,
            load_average_1m: load_avg.one as f32,
            load_average_5m: load_avg.five as f32,
            load_average_15m: load_avg.fifteen as f32,
            frequency_mhz: self.sys.cpus().first().map(|c| c.frequency()).unwrap_or(0),
        })
    }

    fn collect_memory_metrics(&self) -> Result<MemoryMetrics> {
        let total = self.sys.total_memory();
        let used = self.sys.used_memory();
        let available = self.sys.available_memory();
        let swap_total = self.sys.total_swap();
        let swap_used = self.sys.used_swap();

        Ok(MemoryMetrics {
            total_bytes: total,
            used_bytes: used,
            available_bytes: available,
            usage_percent: (used as f32 / total as f32) * 100.0,
            swap_total_bytes: swap_total,
            swap_used_bytes: swap_used,
            swap_usage_percent: if swap_total > 0 {
                (swap_used as f32 / swap_total as f32) * 100.0
            } else {
                0.0
            },
        })
    }

    fn collect_disk_metrics(&self) -> Result<DiskMetrics> {
        let disks = self
            .disks
            .iter()
            .map(|disk| {
                let total = disk.total_space();
                let available = disk.available_space();
                let used = total - available;

                DiskInfo {
                    name: disk.name().to_string_lossy().to_string(),
                    mount_point: disk.mount_point().to_string_lossy().to_string(),
                    total_bytes: total,
                    available_bytes: available,
                    usage_percent: if total > 0 {
                        (used as f32 / total as f32) * 100.0
                    } else {
                        0.0
                    },
                    filesystem: disk.file_system().to_string_lossy().to_string(),
                }
            })
            .collect();

        // Note: sysinfo doesn't provide I/O stats directly
        // We'd need to read from /proc/diskstats for this
        Ok(DiskMetrics {
            disks,
            total_read_bytes: 0,
            total_write_bytes: 0,
        })
    }

    fn collect_thermal_metrics(&self) -> Result<ThermalMetrics> {
        let sensors: Vec<ThermalSensor> = self
            .components
            .iter()
            .map(|component| ThermalSensor {
                name: component.label().to_string(),
                label: component.label().to_string(),
                temperature_celsius: component.temperature(),
                critical_celsius: component.critical(),
            })
            .collect();

        let max_temp = sensors
            .iter()
            .map(|s| s.temperature_celsius)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);

        let avg_temp = if !sensors.is_empty() {
            sensors.iter().map(|s| s.temperature_celsius).sum::<f32>() / sensors.len() as f32
        } else {
            0.0
        };

        Ok(ThermalMetrics {
            sensors,
            max_temp_celsius: max_temp,
            avg_temp_celsius: avg_temp,
        })
    }

    fn collect_network_metrics(&self) -> Result<NetworkMetrics> {
        let interfaces: Vec<NetworkInterface> = self
            .networks
            .iter()
            .map(|(name, data)| NetworkInterface {
                name: name.to_string(),
                rx_bytes: data.received(),
                tx_bytes: data.transmitted(),
                rx_packets: data.packets_received(),
                tx_packets: data.packets_transmitted(),
            })
            .collect();

        let total_rx = interfaces.iter().map(|i| i.rx_bytes).sum();
        let total_tx = interfaces.iter().map(|i| i.tx_bytes).sum();

        Ok(NetworkMetrics {
            interfaces,
            total_rx_bytes: total_rx,
            total_tx_bytes: total_tx,
        })
    }

    /// Check if system is under thermal stress
    pub fn is_thermal_throttling(&self, threshold_celsius: f32) -> bool {
        self.components
            .iter()
            .any(|c| c.temperature() > threshold_celsius)
    }

    /// Check if system is under memory pressure
    pub fn is_memory_pressure(&self, threshold_percent: f32) -> bool {
        let used = self.sys.used_memory() as f32;
        let total = self.sys.total_memory() as f32;
        (used / total) * 100.0 > threshold_percent
    }
}

impl Default for SystemMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_monitor_creation() {
        let monitor = SystemMonitor::new();
        assert!(monitor.sys.cpus().len() > 0);
    }

    #[test]
    fn test_collect_metrics() {
        let mut monitor = SystemMonitor::new();
        let metrics = monitor.collect().unwrap();

        assert!(metrics.cpu.usage_percent >= 0.0);
        assert!(metrics.memory.total_bytes > 0);
    }
}
