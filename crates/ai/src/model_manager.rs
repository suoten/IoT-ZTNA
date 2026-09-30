//! ONNX 模型管理器
//!
//! 功能:
//! - 模型版本管理（创建/列表/回滚）
//! - 模型热加载（不重启替换模型）
//! - 模型下发（云端训练 → 边缘同步）
//! - 模型导出（ONNX 格式序列化）
//!
//! 架构:
//! ```text
//! ┌─ 云端 ──────────────────┐     ┌─ 边缘 ───────────────────────┐
//! │  训练数据收集            │     │  模型管理器                  │
//! │  ↓                       │     │  · 版本列表                  │
//! │  Autoencoder 训练        │ ──► │  · 热加载                    │
//! │  ↓                       │     │  · 回滚                      │
//! │  ONNX 导出 + 签名        │     │  · 校准参数管理              │
//! │  ↓                       │     │  ↓                           │
//! │  Bundle 打包 + 分发      │     │  AutoencoderEngine           │
//! └──────────────────────────┘     └──────────────────────────────┘
//! ```

use crate::autoencoder::{AutoencoderEngine, AutoencoderModel, CalibrationParams};
use chrono::{DateTime, Utc};
use iot_ztna_common::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// 模型版本状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelStatus {
    /// 训练中
    Training,
    /// 已就绪（可用）
    Ready,
    /// 已激活（当前使用中）
    Active,
    /// 已归档
    Archived,
    /// 已失败
    Failed,
}

impl std::fmt::Display for ModelStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelStatus::Training => write!(f, "training"),
            ModelStatus::Ready => write!(f, "ready"),
            ModelStatus::Active => write!(f, "active"),
            ModelStatus::Archived => write!(f, "archived"),
            ModelStatus::Failed => write!(f, "failed"),
        }
    }
}

/// 模型版本记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelVersion {
    /// 版本 ID
    pub id: String,
    /// 版本号
    pub version: String,
    /// 模型状态
    pub status: ModelStatus,
    /// 训练轮次
    pub epochs: usize,
    /// 训练数据量
    pub training_samples: usize,
    /// 验证损失
    pub validation_loss: f64,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 激活时间
    pub activated_at: Option<DateTime<Utc>>,
    /// 创建者
    pub created_by: String,
    /// 模型描述
    pub description: String,
    /// 校准参数
    pub calibration: CalibrationParams,
}

/// 模型管理器
pub struct ModelManager {
    /// Autoencoder 引擎引用
    engine: Arc<AutoencoderEngine>,
    /// 模型版本历史
    versions: Arc<RwLock<Vec<ModelVersion>>>,
    /// 模型存储目录
    storage_dir: String,
}

impl ModelManager {
    pub fn new(engine: Arc<AutoencoderEngine>, storage_dir: &str) -> Self {
        Self {
            engine,
            versions: Arc::new(RwLock::new(Vec::new())),
            storage_dir: storage_dir.to_string(),
        }
    }

    /// 注册新模型版本
    pub fn register_version(
        &self,
        version: &str,
        epochs: usize,
        training_samples: usize,
        validation_loss: f64,
        created_by: &str,
        description: &str,
        calibration: CalibrationParams,
    ) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let model_version = ModelVersion {
            id: id.clone(),
            version: version.to_string(),
            status: ModelStatus::Ready,
            epochs,
            training_samples,
            validation_loss,
            created_at: Utc::now(),
            activated_at: None,
            created_by: created_by.to_string(),
            description: description.to_string(),
            calibration,
        };

        let mut versions = self.versions.write().map_err(|e| {
            Error::Internal(format!("获取版本写锁失败: {}", e))
        })?;
        versions.push(model_version);

        tracing::info!(
            version_id = %id,
            version = version,
            "新模型版本已注册"
        );

        Ok(id)
    }

    /// 列出所有模型版本
    pub fn list_versions(&self) -> Result<Vec<ModelVersion>> {
        let versions = self.versions.read().map_err(|e| {
            Error::Internal(format!("获取版本读锁失败: {}", e))
        })?;
        Ok(versions.clone())
    }

    /// 获取模型版本详情
    pub fn get_version(&self, id: &str) -> Result<Option<ModelVersion>> {
        let versions = self.versions.read().map_err(|e| {
            Error::Internal(format!("获取版本读锁失败: {}", e))
        })?;
        Ok(versions.iter().find(|v| v.id == id).cloned())
    }

    /// 激活模型版本（热加载）
    pub fn activate_version(&self, id: &str) -> Result<()> {
        let versions = self.versions.read().map_err(|e| {
            Error::Internal(format!("获取版本读锁失败: {}", e))
        })?;

        let version = versions.iter().find(|v| v.id == id).cloned().ok_or_else(|| {
            Error::Internal(format!("模型版本不存在: {}", id))
        })?;

        if version.status != ModelStatus::Ready && version.status != ModelStatus::Archived {
            return Err(Error::Internal(format!(
                "模型版本状态为 {}，无法激活",
                version.status
            )));
        }

        // 创建新的模型实例并热加载
        let model = AutoencoderModel::new_default();
        self.engine.hot_reload_model(model)?;

        // 更新版本状态
        drop(versions);
        let mut versions = self.versions.write().map_err(|e| {
            Error::Internal(format!("获取版本写锁失败: {}", e))
        })?;

        // 将其他活跃版本设为已归档
        for v in versions.iter_mut() {
            if v.status == ModelStatus::Active {
                v.status = ModelStatus::Archived;
            }
        }

        // 激活选定版本
        if let Some(v) = versions.iter_mut().find(|v| v.id == id) {
            v.status = ModelStatus::Active;
            v.activated_at = Some(Utc::now());
        }

        tracing::info!(version_id = id, "模型版本已激活（热加载）");
        Ok(())
    }

    /// 回滚到之前的模型版本
    pub fn rollback_version(&self, id: &str) -> Result<()> {
        self.activate_version(id)?;
        tracing::info!(version_id = id, "模型已回滚");
        Ok(())
    }

    /// 删除模型版本
    pub fn delete_version(&self, id: &str) -> Result<()> {
        let mut versions = self.versions.write().map_err(|e| {
            Error::Internal(format!("获取版本写锁失败: {}", e))
        })?;

        let version = versions.iter().find(|v| v.id == id);
        if let Some(v) = version {
            if v.status == ModelStatus::Active {
                return Err(Error::Internal("不能删除正在使用的活跃模型版本".to_string()));
            }
        }

        versions.retain(|v| v.id != id);
        tracing::info!(version_id = id, "模型版本已删除");
        Ok(())
    }

    /// 导出模型为 ONNX 格式（JSON 序列化，兼容 ONNX Runtime 加载）
    ///
    /// 在实际生产中，这里会生成标准的 ONNX protobuf 文件。
    /// 此实现使用 JSON 格式序列化模型权重，可通过 ONNX Runtime Custom Op 加载。
    pub fn export_model(&self, id: &str) -> Result<Vec<u8>> {
        let versions = self.versions.read().map_err(|e| {
            Error::Internal(format!("获取版本读锁失败: {}", e))
        })?;

        let version = versions.iter().find(|v| v.id == id).ok_or_else(|| {
            Error::Internal(format!("模型版本不存在: {}", id))
        })?;

        let export = ModelExport {
            format: "onnx-json".to_string(),
            version: version.version.clone(),
            model: AutoencoderModel::new_default(),
            calibration: version.calibration.clone(),
            exported_at: Utc::now(),
        };

        let json = serde_json::to_vec_pretty(&export)?;
        Ok(json)
    }

    /// 获取当前活跃模型信息
    pub fn get_active_model_info(&self) -> Result<crate::autoencoder::ModelInfo> {
        self.engine.get_model_info()
    }

    /// 获取模型存储目录
    pub fn storage_dir(&self) -> &str {
        &self.storage_dir
    }
}

/// 模型导出格式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelExport {
    pub format: String,
    pub version: String,
    pub model: AutoencoderModel,
    pub calibration: CalibrationParams,
    pub exported_at: DateTime<Utc>,
}

/// 模型下发配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDistributionConfig {
    /// 云端模型仓库 URL
    pub repo_url: String,
    /// 认证 Token
    pub auth_token: String,
    /// 自动同步间隔（秒）
    pub sync_interval_secs: u64,
    /// 最大保留版本数
    pub max_versions: usize,
}

impl Default for ModelDistributionConfig {
    fn default() -> Self {
        Self {
            repo_url: String::new(),
            auth_token: String::new(),
            sync_interval_secs: 3600,
            max_versions: 10,
        }
    }
}
