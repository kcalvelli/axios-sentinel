use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use sentinel_core::client::SentinelClient;
use sentinel_core::config::FleetConfig;

#[derive(Parser)]
#[command(name = "sentinel-cli", about = "Direct access to sentinel-agents over Tailscale")]
struct Cli {
    /// Target host name (or "all" for all hosts)
    host: String,

    #[command(subcommand)]
    command: Commands,

    /// Output raw JSON instead of human-readable format
    #[arg(long, global = true)]
    json: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// System health check
    Health,
    /// System status overview
    Status,
    /// List all services
    Services,
    /// List failed services
    Failed,
    /// Hardware temperatures
    Temperatures,
    /// Disk usage
    Disk,
    /// GPU status
    Gpu,
    /// View journal logs for a unit
    Logs {
        /// Systemd unit name
        unit: String,
        /// Number of lines (default 50)
        #[arg(default_value = "50")]
        lines: u32,
    },
    /// Restart a systemd service (tier 1)
    RestartService {
        /// Systemd unit name
        unit: String,
    },
    /// Trigger GPU reset (tier 1)
    GpuReset,
    /// Vacuum journal (tier 1)
    JournalVacuum {
        /// Max size (e.g., "500M", "1G")
        max_size: String,
    },
    /// Reboot the host (tier 2)
    Reboot,
    /// Kill a process by PID (tier 2)
    KillProcess {
        /// Process ID
        pid: u32,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = FleetConfig::from_env()?;
    let client = SentinelClient::new(config.clone())?;
    let json_output = cli.json;

    if cli.host == "all" {
        run_all(&client, &config, &cli.command, json_output).await
    } else {
        run_single(&client, &cli.host, &cli.command, json_output).await
    }
}

async fn run_single(
    client: &SentinelClient,
    host: &str,
    command: &Commands,
    json_output: bool,
) -> Result<()> {
    match command {
        Commands::Health => {
            let resp = client.health(host).await.context("health check failed")?;
            print_response(&resp, json_output)?;
        }
        Commands::Status => {
            let resp = client.status(host).await.context("status failed")?;
            print_response(&resp, json_output)?;
        }
        Commands::Services => {
            let resp = client.services(host).await.context("services failed")?;
            print_response(&resp, json_output)?;
        }
        Commands::Failed => {
            let resp = client.failed(host).await.context("failed services failed")?;
            print_response(&resp, json_output)?;
        }
        Commands::Temperatures => {
            let resp = client.temperatures(host).await.context("temperatures failed")?;
            print_response(&resp, json_output)?;
        }
        Commands::Disk => {
            let resp = client.disk(host).await.context("disk failed")?;
            print_response(&resp, json_output)?;
        }
        Commands::Gpu => {
            let resp = client.gpu(host).await.context("gpu failed")?;
            print_response(&resp, json_output)?;
        }
        Commands::Logs { unit, lines } => {
            let resp = client
                .logs(host, unit, *lines)
                .await
                .context("logs failed")?;
            if json_output {
                println!("{}", serde_json::to_string_pretty(&resp)?);
            } else if resp.ok {
                if let Some(data) = &resp.data {
                    for line in &data.lines {
                        println!("{line}");
                    }
                }
            } else {
                eprintln!("Error: {}", resp.error.as_deref().unwrap_or("unknown"));
            }
        }
        Commands::RestartService { unit } => {
            let resp = client
                .restart_service(host, unit)
                .await
                .context("restart-service failed")?;
            print_response(&resp, json_output)?;
        }
        Commands::GpuReset => {
            let resp = client.gpu_reset(host).await.context("gpu-reset failed")?;
            print_response(&resp, json_output)?;
        }
        Commands::JournalVacuum { max_size } => {
            let resp = client
                .journal_vacuum(host, max_size)
                .await
                .context("journal-vacuum failed")?;
            print_response(&resp, json_output)?;
        }
        Commands::Reboot => {
            let resp = client.reboot(host).await.context("reboot failed")?;
            print_response(&resp, json_output)?;
            if resp.notify == Some(true) {
                eprintln!("[notify] Tier 2 action taken — notification recommended");
            }
        }
        Commands::KillProcess { pid } => {
            let resp = client
                .kill_process(host, *pid)
                .await
                .context("kill-process failed")?;
            print_response(&resp, json_output)?;
            if resp.notify == Some(true) {
                eprintln!("[notify] Tier 2 action taken — notification recommended");
            }
        }
    }
    Ok(())
}

async fn run_all(
    client: &SentinelClient,
    config: &FleetConfig,
    command: &Commands,
    json_output: bool,
) -> Result<()> {
    match command {
        Commands::Health => {
            let results = client.check_fleet_health().await;
            if json_output {
                println!("{}", serde_json::to_string_pretty(&results.iter().map(|(h, r)| {
                    serde_json::json!({
                        "host": h,
                        "result": match r {
                            Ok(resp) => serde_json::to_value(resp).unwrap_or_default(),
                            Err(e) => serde_json::json!({"error": e.to_string()}),
                        }
                    })
                }).collect::<Vec<_>>())?);
            } else {
                println!("{:<15} {:<8} {}", "HOST", "STATUS", "DETAILS");
                println!("{}", "-".repeat(60));
                for (host, result) in &results {
                    match result {
                        Ok(resp) => {
                            if let Some(data) = &resp.data {
                                let details: Vec<_> = data
                                    .checks
                                    .iter()
                                    .map(|c| format!("{}: {}", c.name, c.message))
                                    .collect();
                                println!(
                                    "{:<15} {:<8} {}",
                                    host,
                                    format!("{:?}", data.status).to_lowercase(),
                                    details.join("; ")
                                );
                            }
                        }
                        Err(e) => {
                            println!("{:<15} {:<8} {}", host, "ERROR", e);
                        }
                    }
                }
            }
        }
        Commands::Status => {
            for host in &config.hosts {
                println!("--- {} ---", host);
                if let Err(e) = run_single(client, host, command, json_output).await {
                    eprintln!("  Error: {e}");
                }
                println!();
            }
        }
        _ => {
            // For other commands on "all", run sequentially
            for host in &config.hosts {
                println!("--- {} ---", host);
                if let Err(e) = run_single(client, host, command, json_output).await {
                    eprintln!("  Error: {e}");
                }
                println!();
            }
        }
    }
    Ok(())
}

fn print_response<T: serde::Serialize>(
    resp: &sentinel_core::types::AgentResponse<T>,
    json_output: bool,
) -> Result<()> {
    if json_output {
        println!("{}", serde_json::to_string_pretty(resp)?);
    } else if resp.ok {
        if let Some(data) = &resp.data {
            println!("{}", serde_json::to_string_pretty(data)?);
        }
    } else {
        eprintln!(
            "Error from {}: {}",
            resp.hostname,
            resp.error.as_deref().unwrap_or("unknown error")
        );
        std::process::exit(1);
    }
    Ok(())
}
