//! Prometheus 指标导出

use axum::{extract::State, response::IntoResponse};
use axum::http::{header, StatusCode};
use axum::body::Body;

use crate::state::AppState;

/// Prometheus 格式指标
pub async fn prometheus_metrics(
    State(state): State<AppState>,
) -> axum::response::Response {
    let online_devices = state.db.count_online_devices().unwrap_or(0);
    let total_devices = state.db.count_devices().unwrap_or(0);
    let unack_alerts = state.db.count_unacknowledged_alerts().unwrap_or(0);
    let denials = state.db.count_denials().unwrap_or(0);

    let metrics = format!(
        r#"# HELP ztna_devices_total 设备总数
# TYPE ztna_devices_total gauge
ztna_devices_total {total_devices}
# HELP ztna_devices_online 在线设备数
# TYPE ztna_devices_online gauge
ztna_devices_online {online_devices}
# HELP ztna_interceptions_total 拦截次数
# TYPE ztna_interceptions_total counter
ztna_interceptions_total {denials}
# HELP ztna_alerts_unacknowledged 未确认告警数
# TYPE ztna_alerts_unacknowledged gauge
ztna_alerts_unacknowledged {unack_alerts}
# HELP ztna_policy_decisions_total 策略决策次数
# TYPE ztna_policy_decisions_total counter
ztna_policy_decisions_total 0
"#
    );

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        Body::from(metrics),
    ).into_response()
}
