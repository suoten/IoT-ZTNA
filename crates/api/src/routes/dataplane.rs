//! 数据面统计 API
//!
//! 提供数据面执行状态和统计信息:
//! - 当前执行模式（eBPF/nftables/memory）
//! - 活跃规则数
//! - 通过/拦截包数和字节数

use axum::{
    extract::State,
    Json,
};
use iot_ztna_common::ApiResponse;

use crate::error::AppResult;
use crate::state::AppState;

/// 获取数据面统计
pub async fn dataplane_stats(
    State(state): State<AppState>,
) -> AppResult<serde_json::Value> {
    let stats = state.dataplane.executor().stats().await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "mode": stats.mode,
        "is_ebpf": state.dataplane.is_ebpf(),
        "active_rules": stats.active_rules,
        "packets_passed": stats.packets_passed,
        "packets_dropped": stats.packets_dropped,
        "bytes_passed": stats.bytes_passed,
    }))))
}

/// 清除所有数据面规则
pub async fn clear_all_rules(
    State(state): State<AppState>,
) -> AppResult<serde_json::Value> {
    state.dataplane.executor().clear_all().await?;

    state.db.audit(
        "admin",
        "clear_dataplane_rules",
        "all",
        "success",
        serde_json::json!({}),
        "",
    )?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "message": "所有数据面规则已清除"
    }))))
}
