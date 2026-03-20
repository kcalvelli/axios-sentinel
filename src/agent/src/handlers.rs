use crate::system;
use crate::AppState;
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use sentinel_core::types::*;
use serde::Deserialize;
use std::sync::Arc;

type AppJson<T> = (StatusCode, Json<AgentResponse<T>>);

fn ok<T>(state: &AppState, data: T) -> AppJson<T> {
    (
        StatusCode::OK,
        Json(AgentResponse::success(state.hostname.clone(), data)),
    )
}

fn ok_notify<T>(state: &AppState, data: T) -> AppJson<T> {
    (
        StatusCode::OK,
        Json(AgentResponse::success_notify(state.hostname.clone(), data)),
    )
}

fn err<T>(state: &AppState, status: StatusCode, msg: impl Into<String>) -> AppJson<T> {
    (
        status,
        Json(AgentResponse::error(state.hostname.clone(), msg)),
    )
}

// -- Read endpoints --

pub async fn health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match system::get_health(&state.hostname) {
        Ok(data) => ok(&state, data),
        Err(e) => err(&state, StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match system::get_status(&state.hostname) {
        Ok(data) => ok(&state, data),
        Err(e) => err(&state, StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn services(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match system::get_services() {
        Ok(data) => ok(&state, data),
        Err(e) => err(&state, StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn failed(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match system::get_failed() {
        Ok(data) => ok(&state, data),
        Err(e) => err(&state, StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn temperatures(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match system::get_temperatures() {
        Ok(data) => ok(&state, data),
        Err(e) => err(&state, StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn disk(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match system::get_disk_usage() {
        Ok(data) => ok(&state, data),
        Err(e) => err(&state, StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn gpu(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match system::get_gpu_status() {
        Ok(data) => ok(&state, data),
        Err(e) => err(&state, StatusCode::NOT_FOUND, e.to_string()),
    }
}

#[derive(Deserialize)]
pub struct LogsQuery {
    lines: Option<u32>,
}

pub async fn logs(
    State(state): State<Arc<AppState>>,
    Path(unit): Path<String>,
    Query(query): Query<LogsQuery>,
) -> impl IntoResponse {
    let lines = query.lines.unwrap_or(50);
    match system::get_logs(&unit, lines) {
        Ok(data) => ok(&state, data),
        Err(e) => err(&state, StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

// -- Tier 1 endpoints --

#[derive(Deserialize)]
pub struct RestartServiceBody {
    unit: String,
}

pub async fn restart_service(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RestartServiceBody>,
) -> impl IntoResponse {
    if !state.tier_config.tier1 {
        return err(&state, StatusCode::FORBIDDEN, "tier 1 operations are disabled on this host");
    }
    if !state.tier_config.restartable_services.contains(&body.unit) {
        return err(
            &state,
            StatusCode::FORBIDDEN,
            format!("service '{}' is not in the restartable services allowlist", body.unit),
        );
    }
    match system::restart_service(&body.unit) {
        Ok(data) => ok(&state, data),
        Err(e) => err(&state, StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn gpu_reset(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    if !state.tier_config.tier1 {
        return err(&state, StatusCode::FORBIDDEN, "tier 1 operations are disabled on this host");
    }
    if !state.tier_config.allow_gpu_reset {
        return err(&state, StatusCode::NOT_FOUND, "GPU reset is not available on this host");
    }
    match system::gpu_reset() {
        Ok(data) => ok(&state, data),
        Err(e) => err(&state, StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

#[derive(Deserialize)]
pub struct JournalVacuumBody {
    max_size: String,
}

pub async fn journal_vacuum(
    State(state): State<Arc<AppState>>,
    Json(body): Json<JournalVacuumBody>,
) -> impl IntoResponse {
    if !state.tier_config.tier1 {
        return err(&state, StatusCode::FORBIDDEN, "tier 1 operations are disabled on this host");
    }
    match system::journal_vacuum(&body.max_size) {
        Ok(data) => ok(&state, data),
        Err(e) => err(&state, StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

// -- Tier 2 endpoints --

pub async fn reboot(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    if !state.tier_config.tier2 {
        return err(&state, StatusCode::FORBIDDEN, "tier 2 operations are disabled on this host");
    }
    match system::reboot() {
        Ok(data) => ok_notify(&state, data),
        Err(e) => err(&state, StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

#[derive(Deserialize)]
pub struct KillProcessBody {
    pid: u32,
}

pub async fn kill_process(
    State(state): State<Arc<AppState>>,
    Json(body): Json<KillProcessBody>,
) -> impl IntoResponse {
    if !state.tier_config.tier2 {
        return err(&state, StatusCode::FORBIDDEN, "tier 2 operations are disabled on this host");
    }
    match system::kill_process(body.pid) {
        Ok(data) => ok_notify(&state, data),
        Err(e) => err(&state, StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}
