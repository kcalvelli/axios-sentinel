mod handlers;
mod system;

use anyhow::{Context, Result};
use axum::{
    Router,
    routing::{get, post},
};
use sentinel_core::config::TierConfig;
use std::sync::Arc;
use tokio::net::TcpListener;

#[derive(Clone)]
pub struct AppState {
    pub hostname: String,
    pub tier_config: TierConfig,
}

#[tokio::main]
async fn main() -> Result<()> {
    let port: u16 = std::env::var("SENTINEL_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(9256);

    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".into());

    let tier_config = TierConfig {
        tier1: std::env::var("SENTINEL_TIER1")
            .map(|v| v != "false" && v != "0")
            .unwrap_or(true),
        tier2: std::env::var("SENTINEL_TIER2")
            .map(|v| v != "false" && v != "0")
            .unwrap_or(true),
        restartable_services: std::env::var("SENTINEL_RESTARTABLE")
            .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default(),
        allow_gpu_reset: std::env::var("SENTINEL_GPU_RESET")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false),
    };

    let state = Arc::new(AppState {
        hostname,
        tier_config,
    });

    // Determine bind address — prefer Tailscale interface
    let bind_addr = resolve_tailscale_ip()
        .unwrap_or_else(|| "0.0.0.0".to_string());

    let app = Router::new()
        // Read endpoints (always available)
        .route("/health", get(handlers::health))
        .route("/status", get(handlers::status))
        .route("/services", get(handlers::services))
        .route("/failed", get(handlers::failed))
        .route("/temperatures", get(handlers::temperatures))
        .route("/disk", get(handlers::disk))
        .route("/gpu", get(handlers::gpu))
        .route("/logs/{unit}", get(handlers::logs))
        // Tier 1 endpoints
        .route("/restart-service", post(handlers::restart_service))
        .route("/gpu-reset", post(handlers::gpu_reset))
        .route("/journal-vacuum", post(handlers::journal_vacuum))
        // Tier 2 endpoints
        .route("/reboot", post(handlers::reboot))
        .route("/kill-process", post(handlers::kill_process))
        .with_state(state);

    let addr = format!("{}:{}", bind_addr, port);
    eprintln!("sentinel-agent listening on {addr}");

    let listener = TcpListener::bind(&addr)
        .await
        .with_context(|| format!("failed to bind to {addr}"))?;

    axum::serve(listener, app).await?;
    Ok(())
}

/// Try to find the Tailscale interface IP.
fn resolve_tailscale_ip() -> Option<String> {
    let output = std::process::Command::new("tailscale")
        .args(["ip", "-4"])
        .output()
        .ok()?;
    if output.status.success() {
        let ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !ip.is_empty() {
            return Some(ip);
        }
    }
    None
}
