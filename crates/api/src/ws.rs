//! WebSocket 实时推送
//!
//! 支持多通道: 设备事件、告警、证书、指标

use crate::state::AppState;
use axum::{
    extract::{Query, State, WebSocketUpgrade},
    response::IntoResponse,
    extract::ws::{Message, WebSocket},
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct WsQuery {
    pub channel: Option<String>,
}

/// WebSocket 入口
pub async fn ws_handler(
    State(state): State<AppState>,
    Query(query): Query<WsQuery>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let channel = query.channel.unwrap_or_else(|| "all".to_string());
    ws.on_upgrade(move |socket| handle_ws(socket, state, channel))
}

async fn handle_ws(socket: WebSocket, state: AppState, channel: String) {
    let (mut sender, mut receiver) = socket.split();

    // 发送欢迎消息
    let hello = serde_json::json!({
        "type": "hello",
        "channel": &channel,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });
    let _ = sender.send(Message::Text(hello.to_string())).await;

    // 根据通道选择订阅
    let mut alert_rx = state.alert_tx.subscribe();
    let mut device_rx = state.device_tx.subscribe();
    let mut metrics_rx = state.metrics_tx.subscribe();

    // 后台推送任务
    let mut send_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                Ok(msg) = alert_rx.recv() => {
                    if channel == "all" || channel == "alerts" {
                        let event = serde_json::json!({
                            "channel": "alerts",
                            "data": serde_json::from_str::<serde_json::Value>(&msg).unwrap_or(serde_json::Value::Null),
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        });
                        if sender.send(Message::Text(event.to_string())).await.is_err() {
                            break;
                        }
                    }
                }
                Ok(msg) = device_rx.recv() => {
                    if channel == "all" || channel == "devices" {
                        let event = serde_json::json!({
                            "channel": "devices",
                            "data": serde_json::from_str::<serde_json::Value>(&msg).unwrap_or(serde_json::Value::Null),
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        });
                        if sender.send(Message::Text(event.to_string())).await.is_err() {
                            break;
                        }
                    }
                }
                Ok(msg) = metrics_rx.recv() => {
                    if channel == "all" || channel == "metrics" {
                        let event = serde_json::json!({
                            "channel": "metrics",
                            "data": serde_json::from_str::<serde_json::Value>(&msg).unwrap_or(serde_json::Value::Null),
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        });
                        if sender.send(Message::Text(event.to_string())).await.is_err() {
                            break;
                        }
                    }
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(30)) => {
                    // 心跳
                    let ping = serde_json::json!({
                        "type": "ping",
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    });
                    if sender.send(Message::Text(ping.to_string())).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    // 接收消息（主要是客户端的 pong/subscribe）
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = &mut send_task => {
            recv_task.abort();
        }
        _ = &mut recv_task => {
            send_task.abort();
        }
    }
}
