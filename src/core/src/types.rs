use serde::{Deserialize, Serialize};

/// Standard response envelope from sentinel-agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse<T> {
    pub ok: bool,
    pub hostname: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notify: Option<bool>,
}

impl<T> AgentResponse<T> {
    pub fn success(hostname: String, data: T) -> Self {
        Self {
            ok: true,
            hostname,
            data: Some(data),
            error: None,
            notify: None,
        }
    }

    pub fn success_notify(hostname: String, data: T) -> Self {
        Self {
            ok: true,
            hostname,
            data: Some(data),
            error: None,
            notify: Some(true),
        }
    }

    pub fn error(hostname: String, message: impl Into<String>) -> Self {
        Self {
            ok: false,
            hostname,
            data: None,
            error: Some(message.into()),
            notify: None,
        }
    }
}

/// System overview data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    pub hostname: String,
    pub uptime_seconds: u64,
    pub load_average: [f64; 3],
    pub memory: MemoryInfo,
    pub swap: SwapInfo,
    pub root_disk_usage_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapInfo {
    pub total_bytes: u64,
    pub used_bytes: u64,
}

/// Systemd service information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub unit: String,
    pub load_state: String,
    pub active_state: String,
    pub sub_state: String,
    pub description: String,
}

/// Hardware temperature reading.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemperatureReading {
    pub device: String,
    pub sensor: String,
    pub current_c: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub high_c: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub critical_c: Option<f64>,
}

/// Disk usage information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskUsage {
    pub filesystem: String,
    pub mount_point: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub use_percent: f64,
}

/// GPU status information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuStatus {
    pub driver: String,
    pub card: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vram_total_mb: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vram_used_mb: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature_c: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub power_watts: Option<f64>,
}

/// Health check result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub status: HealthStatus,
    pub checks: Vec<HealthCheckItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckItem {
    pub name: String,
    pub status: HealthStatus,
    pub message: String,
}

/// Result of a tier 1/2 action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub action: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Journal log lines.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogLines {
    pub unit: String,
    pub lines: Vec<String>,
}
