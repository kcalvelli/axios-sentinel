use crate::config::{FleetConfig, HostEntry};
use crate::types::*;
use anyhow::{Context, Result};
use reqwest::Client;
use std::time::Duration;

/// HTTP client for communicating with sentinel-agents.
pub struct SentinelClient {
    http: Client,
    config: FleetConfig,
}

impl SentinelClient {
    pub fn new(config: FleetConfig) -> Result<Self> {
        let http = Client::builder()
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .context("failed to build HTTP client")?;
        Ok(Self { http, config })
    }

    fn url(&self, host: &str, path: &str) -> String {
        format!("{}{}", self.config.agent_url(host), path)
    }

    /// Get system status.
    pub async fn status(&self, host: &str) -> Result<AgentResponse<SystemStatus>> {
        self.get(host, "/status").await
    }

    /// Get health check.
    pub async fn health(&self, host: &str) -> Result<AgentResponse<HealthCheck>> {
        self.get(host, "/health").await
    }

    /// Get service list.
    pub async fn services(&self, host: &str) -> Result<AgentResponse<Vec<ServiceInfo>>> {
        self.get(host, "/services").await
    }

    /// Get failed services.
    pub async fn failed(&self, host: &str) -> Result<AgentResponse<Vec<ServiceInfo>>> {
        self.get(host, "/failed").await
    }

    /// Get temperatures.
    pub async fn temperatures(&self, host: &str) -> Result<AgentResponse<Vec<TemperatureReading>>> {
        self.get(host, "/temperatures").await
    }

    /// Get disk usage.
    pub async fn disk(&self, host: &str) -> Result<AgentResponse<Vec<DiskUsage>>> {
        self.get(host, "/disk").await
    }

    /// Get GPU status.
    pub async fn gpu(&self, host: &str) -> Result<AgentResponse<GpuStatus>> {
        self.get(host, "/gpu").await
    }

    /// Get journal logs for a unit.
    pub async fn logs(&self, host: &str, unit: &str, lines: u32) -> Result<AgentResponse<LogLines>> {
        self.get(host, &format!("/logs/{}?lines={}", unit, lines))
            .await
    }

    /// Restart a service (tier 1).
    pub async fn restart_service(&self, host: &str, unit: &str) -> Result<AgentResponse<ActionResult>> {
        self.post(host, "/restart-service", &serde_json::json!({"unit": unit}))
            .await
    }

    /// Trigger GPU reset (tier 1).
    pub async fn gpu_reset(&self, host: &str) -> Result<AgentResponse<ActionResult>> {
        self.post(host, "/gpu-reset", &serde_json::json!({})).await
    }

    /// Vacuum journal (tier 1).
    pub async fn journal_vacuum(&self, host: &str, max_size: &str) -> Result<AgentResponse<ActionResult>> {
        self.post(host, "/journal-vacuum", &serde_json::json!({"max_size": max_size}))
            .await
    }

    /// Reboot host (tier 2).
    pub async fn reboot(&self, host: &str) -> Result<AgentResponse<ActionResult>> {
        self.post(host, "/reboot", &serde_json::json!({})).await
    }

    /// Kill a process (tier 2).
    pub async fn kill_process(&self, host: &str, pid: u32) -> Result<AgentResponse<ActionResult>> {
        self.post(host, "/kill-process", &serde_json::json!({"pid": pid}))
            .await
    }

    async fn get<T: serde::de::DeserializeOwned>(&self, host: &str, path: &str) -> Result<AgentResponse<T>> {
        let url = self.url(host, path);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .with_context(|| format!("failed to reach {host} at {url}"))?;
        resp.json()
            .await
            .with_context(|| format!("failed to parse response from {host}"))
    }

    async fn post<T: serde::de::DeserializeOwned>(
        &self,
        host: &str,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<AgentResponse<T>> {
        let url = self.url(host, path);
        let resp = self
            .http
            .post(&url)
            .json(body)
            .send()
            .await
            .with_context(|| format!("failed to reach {host} at {url}"))?;
        resp.json()
            .await
            .with_context(|| format!("failed to parse response from {host}"))
    }

    /// Get the list of configured hosts.
    pub fn hosts(&self) -> &[HostEntry] {
        &self.config.hosts
    }

    /// Check connectivity to all hosts in parallel.
    pub async fn check_fleet_health(&self) -> Vec<(HostEntry, Result<AgentResponse<HealthCheck>>)> {
        let futures: Vec<_> = self
            .config
            .hosts
            .iter()
            .map(|entry| {
                let entry = entry.clone();
                let this = &self;
                async move {
                    let result = this.health(&entry.name).await;
                    (entry, result)
                }
            })
            .collect();

        futures::future::join_all(futures).await
    }
}
