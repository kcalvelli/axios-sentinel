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

/// Host availability class for fleet health evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Availability {
    AlwaysOn,
    Transient,
}

impl Default for Availability {
    fn default() -> Self {
        Self::AlwaysOn
    }
}

/// A host in the fleet with its availability class.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostEntry {
    pub name: String,
    #[serde(default)]
    pub availability: Availability,
}

/// Configuration for connecting to sentinel-agents (used by CLI and MCP server).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetConfig {
    /// Tailnet domain (e.g., "taile0fb4.ts.net").
    pub domain: String,
    /// Hosts to monitor with availability classification.
    pub hosts: Vec<HostEntry>,
    /// Agent port (default 9256).
    pub port: u16,
}

impl FleetConfig {
    /// Load from environment variables, falling back to defaults.
    pub fn from_env() -> anyhow::Result<Self> {
        let domain = std::env::var("SENTINEL_DOMAIN")
            .unwrap_or_else(|_| "taile0fb4.ts.net".to_string());

        let hosts = std::env::var("SENTINEL_HOSTS")
            .map(|h| {
                h.split(',')
                    .map(|s| {
                        let s = s.trim();
                        if let Some((name, class)) = s.split_once(':') {
                            HostEntry {
                                name: name.to_string(),
                                availability: match class {
                                    "transient" => Availability::Transient,
                                    _ => Availability::AlwaysOn,
                                },
                            }
                        } else {
                            HostEntry {
                                name: s.to_string(),
                                availability: Availability::AlwaysOn,
                            }
                        }
                    })
                    .collect()
            })
            .unwrap_or_else(|_| vec![
                HostEntry { name: "edge".into(), availability: Availability::AlwaysOn },
                HostEntry { name: "mini".into(), availability: Availability::AlwaysOn },
            ]);

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

    /// Get just the host names (for callers that don't need availability info).
    pub fn host_names(&self) -> Vec<&str> {
        self.hosts.iter().map(|h| h.name.as_str()).collect()
    }
}
