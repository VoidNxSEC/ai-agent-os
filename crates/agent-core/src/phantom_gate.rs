//! PhantomGate - Bridge para enviar bundles de sistema ao Phantom para julgamento
//!
//! Este módulo implementa o cliente HTTP que se comunica com o Phantom API
//! para enviar métricas de sistema + logs + alertas e receber análises.

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info};

use crate::{system_monitor::SystemMetrics, Alert};
use log_collector::LogEntry;

/// Configuração do PhantomGate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhantomGateConfig {
    /// Habilitar integração com Phantom
    pub enabled: bool,

    /// URL base do Phantom API (ex: http://localhost:8000)
    pub base_url: String,

    /// Timeout para requests HTTP (segundos)
    pub timeout_secs: u64,

    /// Diretório onde bundles serão salvos localmente (backup)
    pub bundle_dir: String,
}

impl Default for PhantomGateConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: "http://localhost:8000".to_string(),
            timeout_secs: 30,
            bundle_dir: "/tmp/phantom-bundles".to_string(),
        }
    }
}

/// Bundle de dados enviado ao Phantom para julgamento
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhantomGateBundle {
    /// Timestamp Unix (segundos)
    pub timestamp: u64,

    /// Hostname do sistema
    pub hostname: Option<String>,

    /// Métricas do sistema
    pub metrics: SystemMetrics,

    /// Lista de alertas detectados
    pub alerts: Vec<Alert>,

    /// Entradas de log recentes (últimas 200)
    pub logs: Vec<LogEntry>,
}

/// Resposta do Phantom após julgar bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhantomGateResult {
    /// Severidade geral: "info", "warning", "critical"
    pub severity: String,

    /// Insights gerados pela análise
    pub insights: Vec<String>,

    /// IDs de ADRs relevantes consultadas
    pub relevant_adrs: Vec<String>,

    /// Recomendações de ações
    pub recommendations: Vec<String>,

    /// Caminho do bundle salvo
    pub bundle_file: String,

    /// Diretório onde bundle foi salvo
    pub bundle_dir: String,

    /// Notas adicionais
    #[serde(default)]
    pub notes: Vec<String>,
}

/// Cliente PhantomGate
pub struct PhantomGate {
    config: PhantomGateConfig,
    client: Client,
}

impl PhantomGate {
    /// Criar novo PhantomGate
    pub fn new(config: PhantomGateConfig) -> Result<Self> {
        // Criar diretório de bundles se não existir
        std::fs::create_dir_all(&config.bundle_dir).context("Failed to create bundle directory")?;

        // Criar cliente HTTP com timeout
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .context("Failed to create HTTP client")?;

        info!(
            "PhantomGate initialized - base_url={} bundle_dir={}",
            config.base_url, config.bundle_dir
        );

        Ok(Self { config, client })
    }

    /// Enviar bundle para Phantom julgar
    pub async fn judge_bundle(&self, bundle: &PhantomGateBundle) -> Result<PhantomGateResult> {
        let url = format!("{}/judge", self.config.base_url);

        debug!(
            "Sending bundle to Phantom: {} alerts, {} logs",
            bundle.alerts.len(),
            bundle.logs.len()
        );

        // Salvar bundle localmente como backup
        let bundle_file = self.save_bundle_locally(bundle)?;

        // Enviar request para Phantom
        let response = self
            .client
            .post(&url)
            .json(bundle)
            .send()
            .await
            .context("Failed to send request to Phantom")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Phantom returned error status {}: {}", status, error_text);
        }

        let mut result: PhantomGateResult = response
            .json()
            .await
            .context("Failed to parse Phantom response")?;

        // Adicionar informações do bundle local
        result.bundle_file = bundle_file.clone();
        result.bundle_dir = self.config.bundle_dir.clone();

        info!(
            "PhantomGate received judgment: severity={} insights={} adrs={}",
            result.severity,
            result.insights.len(),
            result.relevant_adrs.len()
        );

        Ok(result)
    }

    /// Salvar bundle localmente (backup/auditoria)
    fn save_bundle_locally(&self, bundle: &PhantomGateBundle) -> Result<String> {
        use std::fs::File;
        use std::io::Write;

        let filename = format!(
            "{}/bundle-{}.json",
            self.config.bundle_dir, bundle.timestamp
        );

        let json = serde_json::to_string_pretty(bundle).context("Failed to serialize bundle")?;

        let mut file = File::create(&filename).context("Failed to create bundle file")?;

        file.write_all(json.as_bytes())
            .context("Failed to write bundle file")?;

        debug!("Bundle saved locally: {}", filename);

        Ok(filename)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = PhantomGateConfig::default();
        assert_eq!(config.base_url, "http://localhost:8000");
        assert!(!config.enabled);
    }

    #[tokio::test]
    async fn test_phantom_gate_creation() {
        let config = PhantomGateConfig {
            enabled: true,
            bundle_dir: "/tmp/test-phantom-bundles".to_string(),
            ..Default::default()
        };

        let gate = PhantomGate::new(config);
        assert!(gate.is_ok());
    }
}
