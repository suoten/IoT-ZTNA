//! 审计 API

use axum::{
    extract::State,
    Json,
};
use iot_ztna_common::{ApiResponse, AuditLog, PagedResponse, PaginationQuery};

use crate::error::AppResult;
use crate::state::AppState;

/// 列出审计日志
pub async fn list_audit_logs(
    State(state): State<AppState>,
    axum::extract::Query(pagination): axum::extract::Query<PaginationQuery>,
) -> AppResult<PagedResponse<AuditLog>> {
    let page = pagination.page.unwrap_or(1);
    let page_size = pagination.page_size.unwrap_or(50);
    let offset = (page - 1) * page_size;

    let logs = state.db.list_audit_logs(page_size, offset)?;
    let total = logs.len() as u64;

    Ok(Json(ApiResponse::ok(PagedResponse {
        items: logs,
        total,
        page,
        page_size,
    })))
}

/// 导出合规报表（等保 2.0 三级对照）
pub async fn compliance_report(
    State(state): State<AppState>,
) -> AppResult<serde_json::Value> {
    let report = state.db.compliance_report()?;
    Ok(Json(ApiResponse::ok(report)))
}

/// 验证审计日志哈希链完整性
pub async fn verify_audit_integrity(
    State(state): State<AppState>,
) -> AppResult<serde_json::Value> {
    let valid = state.db.verify_audit_chain()?;
    Ok(Json(ApiResponse::ok(serde_json::json!({
        "audit_chain_valid": valid,
        "checked_at": chrono::Utc::now().to_rfc3339(),
    }))))
}
