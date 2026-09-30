//! OPA/Rego 策略引擎 API
//!
//! 功能:
//! - Rego 策略模拟（dry-run）
//! - Bundle 打包/列表/热加载
//! - Rego 默认规则模板
//! - 策略版本查询

use axum::{
    extract::{Path, State},
    Json,
};
use iot_ztna_common::{ApiResponse, PolicyRequest};
use iot_ztna_opa::{RegoEngine, bundle::OpaBundle};
use serde::Deserialize;

use crate::error::AppResult;
use crate::state::AppState;

/// Rego 策略模拟
pub async fn simulate_rego(
    State(state): State<AppState>,
    Json(request): Json<PolicyRequest>,
) -> AppResult<serde_json::Value> {
    let response = state.rego_engine.simulate(&request)?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "decision": response.decision.to_string(),
        "reason": response.reason,
        "matched_rules": response.matched_rules,
        "timestamp": response.timestamp,
    }))))
}

/// 获取默认 Rego 规则模板
pub async fn default_rego_rules() -> AppResult<serde_json::Value> {
    let rules = RegoEngine::default_rego_rules();
    Ok(Json(ApiResponse::ok(serde_json::json!(rules))))
}

/// 获取当前 Rego 引擎版本
pub async fn rego_revision(
    State(state): State<AppState>,
) -> AppResult<serde_json::Value> {
    let revision = state.rego_engine.get_revision()?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "revision": revision,
        "current_bundle": state.bundle_manager.current_revision(),
    }))))
}

/// Bundle 打包请求
#[derive(Debug, Deserialize)]
pub struct PackBundleRequest {
    pub rules: Vec<iot_ztna_opa::engine::RegoRule>,
    pub created_by: String,
}

/// 打包 Bundle
pub async fn pack_bundle(
    State(state): State<AppState>,
    Json(req): Json<PackBundleRequest>,
) -> AppResult<serde_json::Value> {
    let bundle = state.bundle_manager.pack_bundle(req.rules, &req.created_by)?;
    let serialized = state.bundle_manager.serialize(&bundle)?;

    // 存储到数据库
    state.db.insert_opa_bundle(
        &bundle.revision,
        &bundle.metadata.checksum,
        &bundle.metadata.checksum,
        serialized.len(),
    )?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "revision": bundle.revision,
        "rule_count": bundle.metadata.rule_count,
        "checksum": bundle.metadata.checksum,
        "size": serialized.len(),
    }))))
}

/// 热加载 Bundle
pub async fn hot_load_bundle(
    State(state): State<AppState>,
    Path(revision): Path<String>,
) -> AppResult<serde_json::Value> {
    // 从数据库获取活跃 Bundle
    // 在实际生产中，这里会从文件系统或数据库加载指定 revision 的 Bundle
    // 简化：重新打包默认规则
    let rules = RegoEngine::default_rego_rules();
    let bundle = state.bundle_manager.pack_bundle(rules, "hot-load")?;
    state.bundle_manager.hot_load(&bundle)?;

    state.db.audit(
        "admin",
        "hot_load_bundle",
        &revision,
        "success",
        serde_json::json!({"new_revision": bundle.revision}),
        "",
    )?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "revision": bundle.revision,
        "message": "Bundle 已热加载"
    }))))
}

/// 从云端同步 Bundle
pub async fn sync_bundle(
    State(state): State<AppState>,
) -> AppResult<serde_json::Value> {
    let revision = state.bundle_manager.sync_from_cloud("https://opa-repo.local/bundle")?;

    state.db.audit(
        "admin",
        "sync_bundle",
        &revision,
        "success",
        serde_json::json!({"revision": revision}),
        "",
    )?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "revision": revision,
        "message": "默认策略已重新加载并热生效"
    }))))
}
