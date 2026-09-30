//! 集成对接 API（IoT 平台适配、Webhook 同步）

use axum::{
    extract::State,
    Json,
};
use iot_ztna_common::{ApiResponse, DeviceStatus, SpiffeId};
use serde::Deserialize;

use crate::error::AppResult;
use crate::state::AppState;

/// 接收 IoT 平台侧设备变更通知（Webhook）
pub async fn receive_device_event(
    State(state): State<AppState>,
    Json(event): Json<DeviceEvent>,
) -> AppResult<serde_json::Value> {
    tracing::info!(
        "收到平台设备事件: device={}, event_type={}",
        event.spiffe_id,
        event.event_type
    );

    let spiffe_id = SpiffeId::parse(&event.spiffe_id);
    let status = match event.event_type.as_str() {
        "online" => DeviceStatus::Online,
        "offline" => DeviceStatus::Offline,
        "revoked" => DeviceStatus::Revoked,
        _ => {
            tracing::warn!("未知设备事件类型: {}", event.event_type);
            return Ok(Json(ApiResponse::ok(serde_json::json!({
                "received": false,
                "error": "unknown event type"
            }))));
        }
    };

    match &spiffe_id {
        Ok(sid) => {
            state.db.update_device_status(sid, &status)?;
            state.db.audit(
                "platform-webhook",
                "device_event",
                &event.spiffe_id,
                &event.event_type,
                serde_json::json!({"event_type": event.event_type}),
                "",
            )?;
        }
        Err(e) => {
            tracing::warn!("无法解析 SPIFFE ID: {}", e);
        }
    }

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "received": true,
        "spiffe_id": event.spiffe_id,
        "event_type": event.event_type,
    }))))
}

/// 向 IoT 平台同步设备列表
pub async fn sync_devices(
    State(state): State<AppState>,
) -> AppResult<serde_json::Value> {
    let devices = state.db.list_devices(10000, 0)?;
    let synced: Vec<serde_json::Value> = devices
        .iter()
        .map(|d| {
            serde_json::json!({
                "spiffe_id": d.spiffe_id.to_string(),
                "status": d.status.to_string(),
                "trust_level": d.trust_level.to_string(),
                "last_seen": d.last_seen.map(|t| t.to_rfc3339()).unwrap_or_default(),
            })
        })
        .collect();

    tracing::info!("同步 {} 台设备到 IoT 平台", synced.len());

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "synced_count": synced.len(),
        "devices": synced,
    }))))
}

#[derive(Debug, Deserialize)]
pub struct DeviceEvent {
    pub spiffe_id: String,
    pub event_type: String,
    pub timestamp: Option<String>,
}
