use serde::{Deserialize, Serialize};

/// Tier classification for commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    Read,
    Tier1,
    Tier2,
}

/// Read-only commands — always available.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadCommand {
    Health,
    Status,
    Services,
    Failed,
    Temperatures,
    Disk,
    Gpu,
    Logs {
        unit: String,
        lines: Option<u32>,
    },
}

/// Tier 1 commands — autonomous, safe, reversible.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier1Command {
    RestartService { unit: String },
    GpuReset,
    JournalVacuum { max_size: String },
}

/// Tier 2 commands — autonomous but notify the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier2Command {
    Reboot,
    KillProcess { pid: u32 },
}
