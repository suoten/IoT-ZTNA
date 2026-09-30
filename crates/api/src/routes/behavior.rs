//! 行为检测 API

use axum::{
    extract::{Path, State},
    Json,
};
use iot_ztna_common::{ApiResponse, BehaviorAssessment, SpiffeId};

use crate::error::AppResult;
use crate::state::AppState;

/// 评估设备行为
pub async fn assess_behavior(
    State(state): State<AppState>,
    Path(spiffe_str): Path<String>,
) -> AppResult<BehaviorAssessment> {
    let spiffe_id = SpiffeId::parse(&spiffe_str)
        .map_err(|e| iot_ztna_common::Error::SpiffeIdFormat(e.to_string()))?;
    let assessment = state.behavior_engine.assess(&spiffe_id)?;
    Ok(Json(ApiResponse::ok(assessment)))
}

/// 获取行为统计
pub async fn behavior_stats(
    State(state): State<AppState>,
) -> AppResult<serde_json::Value> {
    let stats = state.behavior_engine.get_stats()?;
    Ok(Json(ApiResponse::ok(serde_json::json!({
        "total_events": stats.total_events,
        "devices_with_history": stats.devices_with_history,
        "devices_with_baseline": stats.devices_with_baseline,
    }))))
}

/// 自动学习/更新设备行为基线
pub async fn update_baseline(
    State(state): State<AppState>,
    Path(spiffe_str): Path<String>,
) -> AppResult<serde_json::Value> {
    let spiffe_id = SpiffeId::parse(&spiffe_str)
        .map_err(|e| iot_ztna_common::Error::SpiffeIdFormat(e.to_string()))?;

    state.behavior_engine.update_baseline(&spiffe_id)?;

    // 返回更新后的基线
    let baseline = state.behavior_engine.get_baseline(&spiffe_id)?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "spiffe_id": spiffe_id.to_string(),
        "baseline": baseline,
        "updated_at": chrono::Utc::now().to_rfc3339(),
    }))))
}

/// 获取设备行为基线
pub async fn get_baseline(
    State(state): State<AppState>,
    Path(spiffe_str): Path<String>,
) -> AppResult<serde_json::Value> {
    let spiffe_id = SpiffeId::parse(&spiffe_str)
        .map_err(|e| iot_ztna_common::Error::SpiffeIdFormat(e.to_string()))?;

    let baseline = state.behavior_engine.get_baseline(&spiffe_id)?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "spiffe_id": spiffe_id.to_string(),
        "baseline": baseline,
    }))))
}
