//! 证书 API

use axum::{
    extract::{Path, State},
    Json,
};
use iot_ztna_common::{ApiResponse, Certificate, PagedResponse, PaginationQuery};

use crate::error::AppResult;
use crate::state::AppState;

/// 获取证书吊销列表 (CRL)
pub async fn get_crl(
    State(state): State<AppState>,
) -> AppResult<serde_json::Value> {
    let crl = state.revocation.generate_crl()?;
    Ok(Json(ApiResponse::ok(crl)))
}

/// 检查证书是否已被吊销 (OCSP-like 本地检查)
pub async fn check_revocation(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<CheckRevocationQuery>,
) -> AppResult<serde_json::Value> {
    let revoked = state.revocation.is_cert_revoked(&params.serial_number)?;
    Ok(Json(ApiResponse::ok(serde_json::json!({
        "serial_number": params.serial_number,
        "revoked": revoked,
        "checked_at": chrono::Utc::now().to_rfc3339(),
    }))))
}

#[derive(Debug, serde::Deserialize)]
pub struct CheckRevocationQuery {
    pub serial_number: String,
}

/// 列出证书
pub async fn list_certificates(
    State(state): State<AppState>,
    axum::extract::Query(pagination): axum::extract::Query<PaginationQuery>,
) -> AppResult<PagedResponse<Certificate>> {
    let page = pagination.page.unwrap_or(1);
    let page_size = pagination.page_size.unwrap_or(20);
    let offset = (page - 1) * page_size;

    let certs = state.db.list_certificates(page_size, offset)?;

    // 统计总数
    let total = certs.len() as u64;

    Ok(Json(ApiResponse::ok(PagedResponse {
        items: certs,
        total,
        page,
        page_size,
    })))
}

/// 获取证书详情
pub async fn get_certificate(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Option<Certificate>> {
    let certs = state.db.list_certificates(1000, 0)?;
    let cert = certs.into_iter().find(|c| c.id == id);
    Ok(Json(ApiResponse::ok(cert)))
}

/// 续期证书
pub async fn renew_certificate(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<serde_json::Value> {
    // 查找该证书的 spiffe_id
    let certs = state.db.list_certificates(1000, 0)?;
    let cert = certs
        .into_iter()
        .find(|c| c.id == id)
        .ok_or_else(|| iot_ztna_common::Error::Certificate(format!("证书 {} 不存在", id)))?;

    let svid = state.svid_manager.renew_svid(&cert.spiffe_id)?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "spiffe_id": svid.spiffe_id.to_string(),
        "expires_at": svid.expires_at,
    }))))
}

/// 吊销证书
pub async fn revoke_certificate(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<()> {
    state.db.revoke_certificate(&id)?;
    Ok(Json(ApiResponse::ok(())))
}
