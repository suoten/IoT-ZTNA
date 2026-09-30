//! 模型管理 API
//!
//! 功能:
//! - 模型训练（离线训练 Autoencoder）
//! - 模型版本列表/激活/回滚/删除
//! - 模型热加载
//! - 模型导出（ONNX 格式）
//! - 模型信息查询
//! - 校准参数查询

use axum::{
    extract::{Path, State},
    Json,
};
use iot_ztna_common::ApiResponse;
use iot_ztna_ai::{
    autoencoder::AutoencoderModel,
    model_manager::ModelVersion,
    training::{ModelTrainer, TrainingConfig},
};
use serde::Deserialize;

use crate::error::AppResult;
use crate::state::AppState;

/// 获取当前模型信息
pub async fn get_model_info(
    State(state): State<AppState>,
) -> AppResult<serde_json::Value> {
    let info = state.ae_engine.get_model_info()?;
    Ok(Json(ApiResponse::ok(serde_json::json!({
        "version": info.version,
        "input_dim": info.input_dim,
        "hidden_dims": info.hidden_dims,
        "latent_dim": info.latent_dim,
        "epochs": info.epochs,
        "created_at": info.created_at,
        "calibration": {
            "lower_anchor": info.calibration.lower_anchor,
            "upper_anchor": info.calibration.upper_anchor,
            "calibration_samples": info.calibration.calibration_samples,
            "calibrated_at": info.calibration.calibrated_at,
        }
    }))))
}

/// 列出模型版本
pub async fn list_model_versions(
    State(state): State<AppState>,
) -> AppResult<Vec<ModelVersion>> {
    let versions = state.model_manager.list_versions()?;
    Ok(Json(ApiResponse::ok(versions)))
}

/// 激活模型版本（热加载）
pub async fn activate_model_version(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<serde_json::Value> {
    state.model_manager.activate_version(&id)?;

    state.db.audit(
        "admin",
        "activate_model",
        &id,
        "success",
        serde_json::json!({"version_id": id}),
        "",
    )?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "message": "模型版本已激活"
    }))))
}

/// 回滚模型版本
pub async fn rollback_model_version(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<serde_json::Value> {
    state.model_manager.rollback_version(&id)?;

    state.db.audit(
        "admin",
        "rollback_model",
        &id,
        "success",
        serde_json::json!({"version_id": id}),
        "",
    )?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "message": "模型已回滚"
    }))))
}

/// 删除模型版本
pub async fn delete_model_version(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<()> {
    state.model_manager.delete_version(&id)?;

    state.db.audit(
        "admin",
        "delete_model",
        &id,
        "success",
        serde_json::json!({}),
        "",
    )?;

    Ok(Json(ApiResponse::ok(())))
}

/// 训练请求
#[derive(Debug, Deserialize)]
pub struct TrainRequest {
    pub epochs: Option<usize>,
    pub learning_rate: Option<f64>,
    pub batch_size: Option<usize>,
    pub created_by: String,
    pub description: Option<String>,
}

/// 训练模型
pub async fn train_model(
    State(state): State<AppState>,
    Json(req): Json<TrainRequest>,
) -> AppResult<serde_json::Value> {
    let engine = state.ae_engine.clone();

    // 在后台线程中训练（避免阻塞请求）
    let config = TrainingConfig {
        epochs: req.epochs.unwrap_or(100),
        learning_rate: req.learning_rate.unwrap_or(0.001),
        batch_size: req.batch_size.unwrap_or(32),
        ..Default::default()
    };

    let trainer = ModelTrainer::new(engine.clone());
    let created_by = req.created_by.clone();
    let description = req.description.clone().unwrap_or_else(|| "训练完成".to_string());

    // 生成模拟训练数据（实际生产中从行为历史中提取）
    let training_data = generate_training_data();

    let result = tokio::task::spawn_blocking(move || {
        trainer.train(&training_data, &config)
    })
    .await
    .map_err(|e| iot_ztna_common::Error::Internal(format!("训练任务失败: {}", e)))??;

    // 注册新版本
    let version_str = format!("1.{}", chrono::Utc::now().timestamp() % 1000);
    let cal_lower = result.calibration.lower_anchor;
    let cal_upper = result.calibration.upper_anchor;
    let version_id = state.model_manager.register_version(
        &version_str,
        result.epochs,
        result.training_samples,
        result.validation_loss,
        &created_by,
        &description,
        result.calibration,
    )?;

    // 自动激活新版本
    state.model_manager.activate_version(&version_id)?;

    state.db.audit(
        &created_by,
        "train_model",
        &version_id,
        "success",
        serde_json::json!({
            "version": version_str,
            "final_loss": result.final_loss,
            "validation_loss": result.validation_loss,
            "duration_secs": result.duration_secs,
        }),
        "",
    )?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "version_id": version_id,
        "version": version_str,
        "final_loss": result.final_loss,
        "validation_loss": result.validation_loss,
        "training_samples": result.training_samples,
        "calibration_samples": result.calibration_samples,
        "duration_secs": result.duration_secs,
        "calibration": {
            "lower_anchor": cal_lower,
            "upper_anchor": cal_upper,
        }
    }))))
}

/// 在线微调
pub async fn fine_tune_model(
    State(state): State<AppState>,
    Json(req): Json<TrainRequest>,
) -> AppResult<serde_json::Value> {
    let engine = state.ae_engine.clone();
    let config = TrainingConfig {
        epochs: req.epochs.unwrap_or(50),
        learning_rate: req.learning_rate.unwrap_or(0.0001),
        batch_size: req.batch_size.unwrap_or(16),
        ..Default::default()
    };

    let trainer = ModelTrainer::new(engine);
    let training_data = generate_training_data();

    let result = tokio::task::spawn_blocking(move || {
        trainer.fine_tune(&training_data, &config)
    })
    .await
    .map_err(|e| iot_ztna_common::Error::Internal(format!("微调任务失败: {}", e)))??;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "final_loss": result.final_loss,
        "validation_loss": result.validation_loss,
        "duration_secs": result.duration_secs,
    }))))
}

/// 导出模型（ONNX 格式）
pub async fn export_model(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<serde_json::Value> {
    let data = state.model_manager.export_model(&id)?;
    let base64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "format": "onnx-json",
        "size": data.len(),
        "data": base64,
    }))))
}

/// 生成模拟训练数据（用于演示）
fn generate_training_data() -> Vec<iot_ztna_common::BehaviorEvent> {
    use chrono::Utc;
    use iot_ztna_common::{BehaviorEvent, SpiffeId};

    let mut events = Vec::new();
    let spiffe = SpiffeId::parse("spiffe://iot-ztna.local/device/sensor-001").unwrap();

    for i in 0..500 {
        let hour = (i % 24) as u32;
        let event = BehaviorEvent {
            spiffe_id: spiffe.clone(),
            protocol: if i % 3 == 0 { "modbus".to_string() } else { "mqtt".to_string() },
            operation: if i % 5 == 0 { "write".to_string() } else { "read".to_string() },
            target: format!("10.0.0.{}:502", (i % 10) + 1),
            function_code: Some(((i % 4) + 1) as u8),
            data_size: Some(256 + (i % 100) * 10),
            timestamp: Utc::now() - chrono::Duration::seconds(i as i64),
        };
        events.push(event);
    }

    events
}
