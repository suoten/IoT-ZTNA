//! 策略 API

use axum::{
    extract::{Path, State},
    Json,
};
use iot_ztna_common::{ApiResponse, PagedResponse, PaginationQuery, Policy, PolicyRequest};
use iot_ztna_policy::engine::PolicyRule;
use serde::Deserialize;

use crate::error::AppResult;
use crate::state::AppState;

/// 列出策略
pub async fn list_policies(
    State(state): State<AppState>,
    axum::extract::Query(pagination): axum::extract::Query<PaginationQuery>,
) -> AppResult<PagedResponse<Policy>> {
    let page = pagination.page.unwrap_or(1);
    let page_size = pagination.page_size.unwrap_or(20);
    let offset = (page - 1) * page_size;

    let policies = state.policy_manager.list_policies(page_size, offset)?;
    let total = policies.len() as u64;

    Ok(Json(ApiResponse::ok(PagedResponse {
        items: policies,
        total,
        page,
        page_size,
    })))
}

/// 获取策略
pub async fn get_policy(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Option<Policy>> {
    let policy = state.policy_manager.get_policy(&id)?;
    Ok(Json(ApiResponse::ok(policy)))
}

/// 创建策略
pub async fn create_policy(
    State(state): State<AppState>,
    Json(req): Json<CreatePolicyRequest>,
) -> AppResult<String> {
    let id = state
        .policy_manager
        .create_policy(&req.name, req.rules, &req.created_by)?;
    Ok(Json(ApiResponse::ok(id)))
}

/// 更新策略（版本管理：保存旧版本到历史表，更新当前版本）
pub async fn update_policy(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdatePolicyRequest>,
) -> AppResult<()> {
    let policy = state
        .policy_manager
        .get_policy(&id)?
        .ok_or_else(|| iot_ztna_common::Error::Policy(format!("策略不存在: {}", id)))?;

    // 保存当前版本到历史表
    state.db.save_policy_version(
        &id,
        &policy.version,
        &policy.rego_content,
        &req.updated_by,
    )?;

    // 更新策略内容
    let new_rego = serde_json::to_string_pretty(&req.rules)
        .map_err(|e| iot_ztna_common::Error::Serde(e))?;
    state.db.update_policy_content(&id, &new_rego, &req.updated_by)?;

    // 重新加载策略
    state.policy_manager.init()?;

    // 审计
    state.db.audit(
        &req.updated_by,
        "update_policy",
        &id,
        "success",
        serde_json::json!({"version": policy.version}),
        "",
    )?;

    Ok(Json(ApiResponse::ok(())))
}

#[derive(Debug, Deserialize)]
pub struct CreatePolicyRequest {
    pub name: String,
    pub rules: Vec<PolicyRule>,
    pub created_by: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePolicyRequest {
    pub rules: Vec<PolicyRule>,
    pub updated_by: String,
}

/// 删除策略
pub async fn delete_policy(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<()> {
    state.policy_manager.delete_policy(&id)?;
    Ok(Json(ApiResponse::ok(())))
}

/// 发布策略
pub async fn publish_policy(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<()> {
    state.policy_manager.publish_policy(&id)?;

    // 保存发布版本快照
    if let Some(policy) = state.policy_manager.get_policy(&id)? {
        state.db.save_policy_version(
            &id,
            &policy.version,
            &policy.rego_content,
            "system",
        )?;
    }

    Ok(Json(ApiResponse::ok(())))
}

/// 模拟策略
pub async fn simulate_policy(
    State(state): State<AppState>,
    Path(_id): Path<String>,
    Json(request): Json<PolicyRequest>,
) -> AppResult<serde_json::Value> {
    let response = state.policy_manager.simulate(&request)?;
    Ok(Json(ApiResponse::ok(serde_json::json!({
        "decision": response.decision.to_string(),
        "reason": response.reason,
        "matched_rules": response.matched_rules,
        "timestamp": response.timestamp,
    }))))
}

/// 获取默认规则模板
pub async fn default_rules() -> AppResult<Vec<PolicyRule>> {
    Ok(Json(ApiResponse::ok(
        iot_ztna_policy::engine::PolicyEngine::default_rules(),
    )))
}

/// 获取策略模板库
pub async fn policy_templates() -> AppResult<Vec<iot_ztna_policy::engine::PolicyTemplate>> {
    Ok(Json(ApiResponse::ok(
        iot_ztna_policy::engine::PolicyEngine::policy_templates(),
    )))
}

/// 列出策略版本历史
pub async fn list_policy_versions(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Vec<iot_ztna_storage::PolicyVersion>> {
    let versions = state.db.list_policy_versions(&id)?;
    Ok(Json(ApiResponse::ok(versions)))
}

/// 回滚策略到指定版本
pub async fn rollback_policy(
    State(state): State<AppState>,
    Path((id, version_id)): Path<(String, String)>,
) -> AppResult<()> {
    state.db.rollback_policy(&id, &version_id)?;

    // 重新加载策略
    state.policy_manager.init()?;

    // 审计
    state.db.audit(
        "admin",
        "rollback_policy",
        &id,
        "success",
        serde_json::json!({"version_id": version_id}),
        "",
    )?;

    Ok(Json(ApiResponse::ok(())))
}
