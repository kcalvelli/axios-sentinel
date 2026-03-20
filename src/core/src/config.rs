use serde::{Deserialize, Serialize};

/// Configuration for the sentinel-agent's tier enforcement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierConfig {
    /// Whether tier 1 operations are allowed (restart-service, gpu-reset, journal-vacuum).
    pub tier1: bool,
    /// Whether tier 2 operations are allowed (reboot, kill-process).
    pub tier2: bool,
    /// Services that can be restarted via tier 1.
    pub restartable_services: Vec<String>,
    /// Whether GPU reset is available on this host.
    pub allow_gpu_reset: bool,
}

impl Default for TierConfig {
    fn default() -> Self {
        Self {
            tier1: true,
            tier2: true,
            restartable_services: Vec::new(),
            allow_gpu_reset: false,
        }
    }
}

/// Configuration for connecting to sentinel-agents (used by CLI and MCP server).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetConfig {
    /// Tailnet domain (e.g., "taile0fb4.ts.net").
    pub domain: String,
    /// Host names to monitor.
    pub hosts: Vec<String>,
    /// Agent port (default 9256).
    pub port: u16,
}

impl FleetConfig {
    /// Load from environment variables, falling back to defaults.
    pub fn from_env() -> anyhow::Result<Self> {
        let domain = std::env::var("SENTINEL_DOMAIN")
            .unwrap_or_else(|_| "taile0fb4.ts.net".to_string());

        let hosts = std::env::var("SENTINEL_HOSTS")
            .map(|h| h.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_else(|_| vec!["edge".into(), "mini".into()]);

        let port = std::env::var("SENTINEL_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(9256);

        Ok(Self {
            domain,
            hosts,
            port,
        })
    }

    /// Construct the agent URL for a given host.
    pub fn agent_url(&self, host: &str) -> String {
        format!("http://{}.{}:{}", host, self.domain, self.port)
    }
}
