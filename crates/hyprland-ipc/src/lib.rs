//! Hyprland IPC client for communicating with the Hyprland compositor
//!
//! This crate provides async communication with Hyprland via Unix sockets,
//! supporting both commands and event subscriptions.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

/// Hyprland IPC client
pub struct HyprlandClient {
    socket_path: PathBuf,
}

/// Hyprland workspace info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: i32,
    pub name: String,
    pub monitor: String,
    pub windows: u32,
}

/// Hyprland window info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Window {
    pub address: String,
    pub title: String,
    pub class: String,
    pub workspace: i32,
    pub pid: u32,
}

/// Hyprland events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum HyprlandEvent {
    WorkspaceChanged { id: i32 },
    WindowOpened { address: String },
    WindowClosed { address: String },
    WindowMoved { address: String, workspace: i32 },
    MonitorAdded { name: String },
    MonitorRemoved { name: String },
}

impl HyprlandClient {
    /// Create a new Hyprland IPC client
    pub fn new() -> Result<Self> {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").context("XDG_RUNTIME_DIR not set")?;

        let hyprland_instance = std::env::var("HYPRLAND_INSTANCE_SIGNATURE")
            .context("HYPRLAND_INSTANCE_SIGNATURE not set - not running under Hyprland?")?;

        let socket_path = PathBuf::from(runtime_dir)
            .join("hypr")
            .join(&hyprland_instance)
            .join(".socket.sock");

        if !socket_path.exists() {
            anyhow::bail!("Hyprland socket not found at {:?}", socket_path);
        }

        Ok(Self { socket_path })
    }

    /// Send a command to Hyprland and get the response
    pub async fn dispatch(&self, command: &str) -> Result<String> {
        let mut stream = UnixStream::connect(&self.socket_path)
            .await
            .context("Failed to connect to Hyprland socket")?;

        stream.write_all(command.as_bytes()).await?;
        stream.write_all(b"\n").await?;
        stream.flush().await?;

        let mut reader = BufReader::new(stream);
        let mut response = String::new();
        reader.read_line(&mut response).await?;

        Ok(response.trim().to_string())
    }

    /// Get active workspace info
    pub async fn get_active_workspace(&self) -> Result<Workspace> {
        let response = self.dispatch("j/activeworkspace").await?;
        serde_json::from_str(&response).context("Failed to parse workspace info")
    }

    /// Get all workspaces
    pub async fn get_workspaces(&self) -> Result<Vec<Workspace>> {
        let response = self.dispatch("j/workspaces").await?;
        serde_json::from_str(&response).context("Failed to parse workspaces")
    }

    /// Get all windows
    pub async fn get_clients(&self) -> Result<Vec<Window>> {
        let response = self.dispatch("j/clients").await?;
        serde_json::from_str(&response).context("Failed to parse clients")
    }

    /// Subscribe to Hyprland events
    pub async fn subscribe_events(&self) -> Result<HyprlandEventStream> {
        let events_socket = self.socket_path.parent().unwrap().join(".socket2.sock");

        let stream = UnixStream::connect(&events_socket)
            .await
            .context("Failed to connect to Hyprland events socket")?;

        Ok(HyprlandEventStream {
            reader: BufReader::new(stream),
        })
    }
}

impl Default for HyprlandClient {
    fn default() -> Self {
        Self::new().expect("Failed to create Hyprland client")
    }
}

/// Stream of Hyprland events
pub struct HyprlandEventStream {
    reader: BufReader<UnixStream>,
}

impl HyprlandEventStream {
    /// Read next event from stream
    pub async fn next_event(&mut self) -> Result<Option<HyprlandEvent>> {
        let mut line = String::new();
        let bytes_read = self.reader.read_line(&mut line).await?;

        if bytes_read == 0 {
            return Ok(None);
        }

        // Parse Hyprland event format: "EVENT>>data"
        let parts: Vec<&str> = line.trim().splitn(2, ">>").collect();
        if parts.len() != 2 {
            return Ok(None);
        }

        let event = match parts[0] {
            "workspace" => HyprlandEvent::WorkspaceChanged {
                id: parts[1].parse().unwrap_or(0),
            },
            "openwindow" => HyprlandEvent::WindowOpened {
                address: parts[1].to_string(),
            },
            "closewindow" => HyprlandEvent::WindowClosed {
                address: parts[1].to_string(),
            },
            "movewindow" => {
                let data: Vec<&str> = parts[1].split(',').collect();
                HyprlandEvent::WindowMoved {
                    address: data.get(0).unwrap_or(&"").to_string(),
                    workspace: data.get(1).and_then(|s| s.parse().ok()).unwrap_or(0),
                }
            }
            "monitoradded" => HyprlandEvent::MonitorAdded {
                name: parts[1].to_string(),
            },
            "monitorremoved" => HyprlandEvent::MonitorRemoved {
                name: parts[1].to_string(),
            },
            _ => return Ok(None), // Unknown event, skip
        };

        Ok(Some(event))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        // This will fail if not running under Hyprland
        let result = HyprlandClient::new();
        if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
            assert!(result.is_ok());
        } else {
            assert!(result.is_err());
        }
    }
}
