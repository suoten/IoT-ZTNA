//! 策略审批 API
//!
//! 多级审批流程:
//! 1. 操作员创建/修改策略 → 自动提交审批请求
//! 2. 管理员审批 → 批准则自动发布，拒绝则驳回
//! 3. 审批记录全量留痕

use axum::{
    extract::{Path, State},
    Json,
};
use iot_ztna_common::{ApiResponse, PaginationQuery};
use iot_ztna_storage::PolicyApproval;
use serde::Deserialize;

use crate::error::AppResult;
use crate::state::AppState;

/// 列出审批请求
pub async fn list_approvals(
    State(state): State<AppState>,
    axum::extract::Query(pagination): axum::extract::Query<PaginationQuery>,
) -> AppResult<serde_json::Value> {
    let page = pagination.page.unwrap_or(1);
    let page_size = pagination.page_size.unwrap_or(20);

    let db = state.db.clone();
    let pending_only = false; // 列出所有

    let approvals = tokio::task::spawn_blocking(move || -> iot_ztna_common::Result<Vec<PolicyApproval>> {
        if pending_only {
            db.list_pending_approvals()
        } else {
            db.list_all_approvals()
        }
    })
    .await
    .map_err(|e| iot_ztna_common::Error::Internal(format!("查询审批任务失败: {}", e)))??;

    let total = approvals.len();
    let start = ((page - 1) * page_size) as usize;
    let end = (start + page_size as usize).min(total);
    let items = if start < total {
        approvals[start..end].to_vec()
    } else {
        vec![]
    };

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "items": items,
        "total": total,
        "page": page,
        "page_size": page_size,
    }))))
}

/// 列出待审批
pub async fn list_pending(
    State(state): State<AppState>,
) -> AppResult<Vec<PolicyApproval>> {
    let db = state.db.clone();
    let approvals = tokio::task::spawn_blocking(move || db.list_pending_approvals())
        .await
        .map_err(|e| iot_ztna_common::Error::Internal(format!("查询待审批任务失败: {}", e)))??;

    Ok(Json(ApiResponse::ok(approvals)))
}

/// 获取审批详情
pub async fn get_approval(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Option<PolicyApproval>> {
    let db = state.db.clone();
    let approval = tokio::task::spawn_blocking(move || db.get_policy_approval(&id))
        .await
        .map_err(|e| iot_ztna_common::Error::Internal(format!("查询审批详情失败: {}", e)))??;

    Ok(Json(ApiResponse::ok(approval)))
}

/// 审批策略
#[derive(Debug, Deserialize)]
pub struct ReviewRequest {
    pub approved: bool,
    pub reviewed_by: String,
    pub comment: String,
}

pub async fn review_approval(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<ReviewRequest>,
) -> AppResult<serde_json::Value> {
    let db = state.db.clone();
    let approval_id = id.clone();
    let reviewed_by = req.reviewed_by.clone();
    let comment = req.comment.clone();
    let approved = req.approved;

    // 执行审批
    tokio::task::spawn_blocking(move || -> iot_ztna_common::Result<()> {
        db.review_policy_approval(&approval_id, approved, &reviewed_by, &comment)
    })
    .await
    .map_err(|e| iot_ztna_common::Error::Internal(format!("审批任务失败: {}", e)))??;

    // 如果审批通过，发布策略
    if req.approved {
        let db2 = state.db.clone();
        let approval_id2 = id.clone();
        tokio::task::spawn_blocking(move || -> iot_ztna_common::Result<()> {
            // 获取审批详情，找到对应的 policy_id
            if let Some(approval) = db2.get_policy_approval(&approval_id2)? {
                if !approval.policy_id.is_empty() {
                    // 发布策略
                    state.policy_manager.publish_policy(&approval.policy_id)?;
                }
            }
            Ok(())
        })
        .await
        .map_err(|e| iot_ztna_common::Error::Internal(format!("发布策略失败: {}", e)))??;
    }

    // 审计日志
    let action = if req.approved { "approve_policy" } else { "reject_policy" };
    state.db.audit(
        &req.reviewed_by,
        action,
        &id,
        if req.approved { "approved" } else { "rejected" },
        serde_json::json!({"comment": req.comment}),
        "",
    )?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "approved": req.approved,
        "message": if req.approved { "策略已审批通过并发布" } else { "策略已被拒绝" }
    }))))
}

/// 提交策略审批
#[derive(Debug, Deserialize)]
pub struct SubmitApprovalRequest {
    pub policy_id: String,
    pub policy_name: String,
    pub rego_content: String,
    pub submitted_by: String,
}

pub async fn submit_approval(
    State(state): State<AppState>,
    Json(req): Json<SubmitApprovalRequest>,
) -> AppResult<String> {
    let db = state.db.clone();
    let policy_id = req.policy_id.clone();
    let policy_name_for_audit = req.policy_name.clone();
    let policy_name = req.policy_name.clone();
    let rego_content = req.rego_content.clone();
    let submitted_by_for_audit = req.submitted_by.clone();
    let submitted_by = req.submitted_by.clone();

    let approval_id = tokio::task::spawn_blocking(move || -> iot_ztna_common::Result<String> {
        db.create_policy_approval(&policy_id, &policy_name, &rego_content, &submitted_by)
    })
    .await
    .map_err(|e| iot_ztna_common::Error::Internal(format!("提交审批任务失败: {}", e)))??;

    // 审计
    state.db.audit(
        &submitted_by_for_audit,
        "submit_policy_approval",
        &approval_id,
        "pending",
        serde_json::json!({"policy_name": policy_name_for_audit}),
        "",
    )?;

    Ok(Json(ApiResponse::ok(approval_id)))
}
