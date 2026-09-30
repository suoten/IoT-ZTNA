//! CA 管理 API

use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Json,
};
use iot_ztna_common::ApiResponse;

use crate::error::AppResult;
use crate::state::AppState;

/// 下载 CA 证书 (PEM 格式)
pub async fn download_ca_cert(State(_state): State<AppState>) -> Response {
    let cert_dir = std::env::var("ZTNA_CERT_DIR").unwrap_or_else(|_| "data/certs".to_string());
    let cert_path = format!("{}/ca.crt", cert_dir);
    let pem = match std::fs::read_to_string(&cert_path) {
        Ok(content) => content,
        Err(e) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::err("CA_CERT_READ_ERROR", &format!("读取 CA 证书失败: {}", e))),
            )
                .into_response()
        }
    };
    (
        axum::http::StatusCode::OK,
        [
            (axum::http::header::CONTENT_TYPE, "application/x-pem-file"),
            (
                axum::http::header::CONTENT_DISPOSITION,
                "attachment; filename=\"ca.crt\"",
            ),
        ],
        pem,
    )
        .into_response()
}

/// 重新生成 Root CA
pub async fn regenerate_ca(State(_state): State<AppState>) -> AppResult<()> {
    let trust_domain = std::env::var("ZTNA_TRUST_DOMAIN")
        .unwrap_or_else(|_| "iot-ztna.local".to_string());
    let cert_dir = std::env::var("ZTNA_CERT_DIR").unwrap_or_else(|_| "data/certs".to_string());

    let new_ca = iot_ztna_identity::CertificateAuthority::generate(&trust_domain)
        .map_err(|e| iot_ztna_common::Error::Certificate(format!("重新生成 CA 失败: {}", e)))?;

    new_ca
        .save_to_dir(std::path::Path::new(&cert_dir))
        .map_err(|e| iot_ztna_common::Error::Certificate(format!("保存 CA 失败: {}", e)))?;

    tracing::warn!("Root CA 已重新生成，需要重启服务以应用更改");

    Ok(Json(ApiResponse::ok(())))
}

/// 备份数据库
pub async fn backup_database(State(_state): State<AppState>) -> AppResult<serde_json::Value> {
    let db_path = std::env::var("ZTNA_DB_PATH").unwrap_or_else(|_| "data/ztna.db".to_string());
    let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let filename = format!("ztna-backup-{}.db", timestamp);
    let backup_path = format!("data/backups/{}", filename);
    let backup_size = std::fs::metadata(&db_path)
        .map(|m| m.len())
        .unwrap_or(0);

    // 确保备份目录存在
    if let Some(parent) = std::path::Path::new(&backup_path).parent() {
        std::fs::create_dir_all(parent).map_err(iot_ztna_common::Error::Io)?;
    }

    std::fs::copy(&db_path, &backup_path)
        .map_err(|e| iot_ztna_common::Error::Internal(format!("数据库备份失败: {}", e)))?;

    tracing::info!("数据库已备份到 {}", backup_path);

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "path": backup_path,
        "filename": filename,
        "size": backup_size,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))))
}

/// 列出所有备份文件
pub async fn list_backups(State(_state): State<AppState>) -> AppResult<Vec<serde_json::Value>> {
    let backup_dir = "data/backups";
    let mut backups: Vec<serde_json::Value> = vec![];

    if let Ok(entries) = std::fs::read_dir(backup_dir) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    let filename = entry.file_name().to_string_lossy().to_string();
                    let size = metadata.len();
                    let modified = metadata.modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| chrono::DateTime::from_timestamp(d.as_secs() as i64, 0))
                        .flatten()
                        .map(|dt| dt.to_rfc3339())
                        .unwrap_or_default();
                    backups.push(serde_json::json!({
                        "filename": filename,
                        "size": size,
                        "modified": modified,
                    }));
                }
            }
        }
    }

    // 按修改时间倒序
    backups.sort_by(|a, b| {
        let a_time = a["modified"].as_str().unwrap_or("");
        let b_time = b["modified"].as_str().unwrap_or("");
        b_time.cmp(a_time)
    });

    Ok(Json(ApiResponse::ok(backups)))
}

/// 下载备份文件
pub async fn download_backup(
    State(_state): State<AppState>,
    axum::extract::Path(filename): axum::extract::Path<String>,
) -> Response {
    // 安全检查：防止路径穿越
    if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::err("INVALID_FILENAME", "无效的文件名")),
        )
            .into_response();
    }

    let backup_path = format!("data/backups/{}", filename);

    match std::fs::read(&backup_path) {
        Ok(data) => {
            (
                axum::http::StatusCode::OK,
                [
                    (axum::http::header::CONTENT_TYPE, "application/octet-stream"),
                    (
                        axum::http::header::CONTENT_DISPOSITION,
                        &*format!("attachment; filename=\"{}\"", filename),
                    ),
                ],
                data,
            )
                .into_response()
        }
        Err(_) => (
            axum::http::StatusCode::NOT_FOUND,
            Json(ApiResponse::<()>::err("BACKUP_NOT_FOUND", "备份文件不存在")),
        )
            .into_response(),
    }
}
