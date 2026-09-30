//! 设备 API

use axum::{
    extract::{Path, State},
    Json,
};
use iot_ztna_common::{
    ApiResponse, Device, DeviceRegistrationRequest, PagedResponse,
    PaginationQuery,
};
use serde::Deserialize;

use crate::error::AppResult;
use crate::state::AppState;

/// 列出设备
pub async fn list_devices(
    State(state): State<AppState>,
    axum::extract::Query(pagination): axum::extract::Query<PaginationQuery>,
) -> AppResult<PagedResponse<Device>> {
    let page = pagination.page.unwrap_or(1);
    let page_size = pagination.page_size.unwrap_or(20);
    let offset = (page - 1) * page_size;

    let devices = state
        .db
        .list_devices(page_size, offset)?;
    let total = state.db.count_devices()?;

    Ok(Json(ApiResponse::ok(PagedResponse {
        items: devices,
        total,
        page,
        page_size,
    })))
}

/// 获取设备详情
pub async fn get_device(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Option<Device>> {
    // 尝试按序列号查询
    let device = state.db.get_device_by_serial(&id)?;
    Ok(Json(ApiResponse::ok(device)))
}

/// 注册设备
pub async fn register_device(
    State(state): State<AppState>,
    Json(req): Json<DeviceRegistrationRequest>,
) -> AppResult<Device> {
    let (device, _svid) = state
        .registration
        .register_device(req, &state.svid_manager)?;

    // 广播设备上线事件
    let _ = state.device_tx.send(serde_json::json!({
        "event": "device_registered",
        "spiffe_id": device.spiffe_id.to_string(),
        "serial_number": device.serial_number,
    }).to_string());

    Ok(Json(ApiResponse::ok(device)))
}

/// 删除设备
pub async fn delete_device(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<()> {
    let device = state
        .db
        .get_device_by_serial(&id)?
        .ok_or_else(|| iot_ztna_common::Error::DeviceNotFound(id.clone()))?;

    state.revocation.delete_device(&device.spiffe_id)?;

    Ok(Json(ApiResponse::ok(())))
}

/// 吊销设备
pub async fn revoke_device(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<()> {
    let device = state
        .db
        .get_device_by_serial(&id)?
        .ok_or_else(|| iot_ztna_common::Error::DeviceNotFound(id.clone()))?;

    state
        .revocation
        .revoke_device(&device.spiffe_id, "通过 API 吊销")?;

    // 广播告警
    let _ = state.alert_tx.send(serde_json::json!({
        "event": "device_revoked",
        "spiffe_id": device.spiffe_id.to_string(),
        "severity": "critical",
    }).to_string());

    Ok(Json(ApiResponse::ok(())))
}

/// 恢复设备
pub async fn reactivate_device(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<()> {
    let device = state
        .db
        .get_device_by_serial(&id)?
        .ok_or_else(|| iot_ztna_common::Error::DeviceNotFound(id.clone()))?;

    state.revocation.reactivate_device(&device.spiffe_id)?;

    Ok(Json(ApiResponse::ok(())))
}

/// 设置维护模式
pub async fn set_maintenance(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<MaintenanceRequest>,
) -> AppResult<()> {
    let device = state
        .db
        .get_device_by_serial(&id)?
        .ok_or_else(|| iot_ztna_common::Error::DeviceNotFound(id.clone()))?;

    state
        .db
        .set_maintenance_mode(&device.spiffe_id, body.enabled)?;

    Ok(Json(ApiResponse::ok(())))
}

#[derive(Debug, Deserialize)]
pub struct MaintenanceRequest {
    pub enabled: bool,
}
