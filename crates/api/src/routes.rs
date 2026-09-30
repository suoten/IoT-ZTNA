//! API 路由定义

mod device;
mod certificate;
mod policy;
mod alert;
mod audit;
mod health;
mod metrics;
mod behavior;
mod integration;
mod ca;
pub mod user;
mod approval;
mod model;
mod dataplane;
mod opa;

// Re-export error types for route handlers
pub use crate::error::AppResult;

use axum::{
    Router,
    routing::{get, post},
    extract::State,
    Json,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tower_http::services::{ServeDir, ServeFile};

use crate::auth::{LoginRequest, LoginResponse};
use crate::state::AppState;
use iot_ztna_common::ApiResponse;

/// 登录接口
async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> AppResult<LoginResponse> {
    let db = state.db.clone();
    let username = req.username.clone();
    let password = req.password.clone();

    // 在阻塞线程中执行数据库查询 + 密码验证
    let result = tokio::task::spawn_blocking(move || -> Result<(String, String), iot_ztna_common::Error> {
        let conn = db.conn()?;

        let row = conn.query_row(
            "SELECT id, password_hash, role, status FROM users WHERE username = ?1",
            rusqlite::params![username],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        );

        let (user_id, stored_hash, role, status) = match row {
            Ok(v) => v,
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                return Err(iot_ztna_common::Error::Auth("用户名或密码错误".to_string()));
            }
            Err(e) => {
                return Err(iot_ztna_common::Error::Database(format!("查询用户失败: {}", e)));
            }
        };

        if status != "active" {
            return Err(iot_ztna_common::Error::Auth("账户已禁用，请联系管理员".to_string()));
        }

        if !crate::routes::user::verify_password_pub(&password, &stored_hash) {
            return Err(iot_ztna_common::Error::Auth("用户名或密码错误".to_string()));
        }

        // 更新最后登录时间
        let now = chrono::Utc::now().to_rfc3339();
        let _ = conn.execute(
            "UPDATE users SET last_login = ?1 WHERE id = ?2",
            rusqlite::params![now, user_id],
        );

        Ok((user_id, role))
    }).await.map_err(|e| iot_ztna_common::Error::Internal(format!("登录任务失败: {}", e)))?;

    let (_user_id, role) = result?;

    let token = state.auth_config
        .sign_token(&req.username, &role, 24)
        .map_err(|e| iot_ztna_common::Error::Internal(format!("JWT 签发失败: {}", e)))?;
    Ok(Json(ApiResponse::ok(LoginResponse {
        token,
        expires_in: 86400,
        role,
    })))
}

/// 创建 API 路由
pub fn create_router(state: AppState) -> Router {
    // 生产级 CORS：允许特定来源而非 permissive
    let cors = CorsLayer::new()
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers(Any)
        .allow_origin(Any); // 生产环境应配置为具体前端域名

    let api_v1 = Router::new()
        // 认证
        .route("/auth/login", post(login))
        // 设备管理
        .route("/devices", get(device::list_devices).post(device::register_device))
        .route("/devices/:id", get(device::get_device).delete(device::delete_device))
        .route("/devices/:id/revoke", axum::routing::post(device::revoke_device))
        .route("/devices/:id/reactivate", axum::routing::post(device::reactivate_device))
        .route("/devices/:id/maintenance", axum::routing::post(device::set_maintenance))
        // 证书管理
        .route("/certificates", get(certificate::list_certificates))
        .route("/certificates/crl", get(certificate::get_crl))
        .route("/certificates/revocation-check", get(certificate::check_revocation))
        .route("/certificates/:id", get(certificate::get_certificate))
        .route("/certificates/:id/renew", axum::routing::post(certificate::renew_certificate))
        .route("/certificates/:id/revoke", axum::routing::post(certificate::revoke_certificate))
        // 策略管理 - 注意: 静态路由必须在动态路由之前
        .route("/policies", get(policy::list_policies).post(policy::create_policy))
        .route("/policies/default-rules", get(policy::default_rules))
.route("/policies/templates", get(policy::policy_templates))
        .route("/policies/:id", get(policy::get_policy).put(policy::update_policy).delete(policy::delete_policy))
        .route("/policies/:id/publish", axum::routing::post(policy::publish_policy))
        .route("/policies/:id/simulate", axum::routing::post(policy::simulate_policy))
        .route("/policies/:id/versions", get(policy::list_policy_versions))
        .route("/policies/:id/rollback/:version_id", axum::routing::post(policy::rollback_policy))
        // 策略审批流程
        .route("/approvals", get(approval::list_approvals).post(approval::submit_approval))
        .route("/approvals/pending", get(approval::list_pending))
        .route("/approvals/:id", get(approval::get_approval))
        .route("/approvals/:id/review", axum::routing::post(approval::review_approval))
        // 告警
        .route("/alerts", get(alert::list_alerts))
        .route("/alerts/:id/ack", axum::routing::post(alert::acknowledge_alert))
        // 审计
        .route("/audit/logs", get(audit::list_audit_logs))
        .route("/audit/compliance-report", get(audit::compliance_report))
        .route("/audit/verify", get(audit::verify_audit_integrity))
        // 健康 & 指标
        .route("/health", get(health::health_check))
        .route("/metrics", get(metrics::prometheus_metrics))
        // 行为检测 - 注意: 静态路由必须在动态路由之前
        .route("/behavior/stats", get(behavior::behavior_stats))
        .route("/behavior/:spiffe_id/assess", get(behavior::assess_behavior))
        .route("/behavior/:spiffe_id/baseline", get(behavior::get_baseline))
        .route("/behavior/:spiffe_id/update-baseline", axum::routing::post(behavior::update_baseline))
        // 数据面统计
        .route("/dataplane/stats", get(dataplane::dataplane_stats))
        .route("/dataplane/clear", axum::routing::post(dataplane::clear_all_rules))
        // 模型管理
        .route("/models/info", get(model::get_model_info))
        .route("/models/versions", get(model::list_model_versions))
        .route("/models/train", axum::routing::post(model::train_model))
        .route("/models/fine-tune", axum::routing::post(model::fine_tune_model))
        .route("/models/:id/activate", axum::routing::post(model::activate_model_version))
        .route("/models/:id/rollback", axum::routing::post(model::rollback_model_version))
        .route("/models/:id", axum::routing::delete(model::delete_model_version))
        .route("/models/:id/export", get(model::export_model))
        // OPA/Rego 策略引擎
        .route("/opa/simulate", axum::routing::post(opa::simulate_rego))
        .route("/opa/default-rules", get(opa::default_rego_rules))
        .route("/opa/revision", get(opa::rego_revision))
        .route("/opa/bundle/pack", axum::routing::post(opa::pack_bundle))
        .route("/opa/bundle/hot-load/:revision", axum::routing::post(opa::hot_load_bundle))
        .route("/opa/bundle/sync", axum::routing::post(opa::sync_bundle))
        // 集成对接
        .route("/integrations/device-events", axum::routing::post(integration::receive_device_event))
        .route("/integrations/sync-devices", axum::routing::post(integration::sync_devices))
        // CA 管理
        .route("/ca/cert", get(ca::download_ca_cert))
        .route("/ca/regenerate", axum::routing::post(ca::regenerate_ca))
        .route("/ca/backup", axum::routing::post(ca::backup_database))
.route("/ca/backups", get(ca::list_backups))
.route("/ca/backups/:filename", get(ca::download_backup))
        // 用户管理
        .route("/users", get(user::list_users).post(user::create_user))
        .route("/users/:id", axum::routing::put(user::update_user).delete(user::delete_user))
        .route("/users/:id/password", axum::routing::post(user::change_password))
        .route("/users/:id/reset-password", axum::routing::post(user::reset_password));

    // 前端静态文件服务（SPA fallback: 找不到文件时返回 index.html）
    let static_dir = std::env::var("ZTNA_STATIC_DIR").unwrap_or_else(|_| "console/dist".to_string());
    let index_file = format!("{}/index.html", &static_dir);
    let serve_dir = ServeDir::new(&static_dir)
        .append_index_html_on_directories(true)
        .fallback(ServeFile::new(index_file));

    Router::new()
        .route("/ws", get(crate::ws::ws_handler))
        .nest("/api/v1", api_v1)
        .fallback_service(serve_dir)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .layer(axum::middleware::from_fn_with_state(
            state.auth_config.clone(),
            crate::auth::jwt_auth_middleware,
        ))
        .with_state(state)
}
