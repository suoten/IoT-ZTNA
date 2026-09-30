//! 健康检查 API

use axum::{extract::State, Json};
use iot_ztna_common::ApiResponse;
use serde::Serialize;

use crate::error::AppResult;
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub gateway_id: String,
    pub components: ComponentHealth,
}

#[derive(Debug, Serialize)]
pub struct ComponentHealth {
    pub database: String,
    pub policy_engine: String,
    pub svid_manager: String,
}

/// 健康检查
pub async fn health_check(
    State(state): State<AppState>,
) -> AppResult<HealthStatus> {
    let db_ok = state.db.count_devices().is_ok();

    let health = HealthStatus {
        status: if db_ok { "healthy".to_string() } else { "degraded".to_string() },
        gateway_id: state.config.gateway_id.clone(),
        components: ComponentHealth {
            database: if db_ok { "up".to_string() } else { "down".to_string() },
            policy_engine: "up".to_string(),
            svid_manager: "up".to_string(),
        },
    };

    Ok(Json(ApiResponse::ok(health)))
}
