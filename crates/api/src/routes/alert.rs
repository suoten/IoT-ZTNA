//! 告警 API

use axum::{
    extract::{Path, State},
    Json,
};
use iot_ztna_common::{Alert, ApiResponse, PagedResponse, PaginationQuery};

use crate::error::AppResult;
use crate::state::AppState;

/// 列出告警
pub async fn list_alerts(
    State(state): State<AppState>,
    axum::extract::Query(pagination): axum::extract::Query<PaginationQuery>,
) -> AppResult<PagedResponse<Alert>> {
    let page = pagination.page.unwrap_or(1);
    let page_size = pagination.page_size.unwrap_or(20);
    let offset = (page - 1) * page_size;

    let alerts = state.db.list_alerts(page_size, offset)?;
    let total = alerts.len() as u64;

    Ok(Json(ApiResponse::ok(PagedResponse {
        items: alerts,
        total,
        page,
        page_size,
    })))
}

/// 确认告警
pub async fn acknowledge_alert(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<()> {
    state.db.acknowledge_alert(&id)?;
    Ok(Json(ApiResponse::ok(())))
}
