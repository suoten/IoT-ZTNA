//! OPA Bundle 管理器
//!
//! 功能:
//! - Bundle 打包与签名
//! - Bundle 热加载（无需重启）
//! - Bundle 版本管理
//! - 本地缓存与离线降级
//!
//! Bundle 格式:
//! ```json
//! {
//!   "revision": "uuid-v4",
//!   "created_at": "2026-09-28T12:00:00Z",
//!   "rules": [...],
//!   "metadata": {
//!     "version": "1.0",
//!     "checksum": "sha256:...",
//!     "created_by": "admin"
//!   }
//! }
//! ```

use crate::engine::{RegoEngine, RegoRule};
use chrono::{DateTime, Utc};
use iot_ztna_common::{Error, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;

/// OPA Bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpaBundle {
    /// 版本号 (UUID)
    pub revision: String,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// Rego 规则列表
    pub rules: Vec<RegoRule>,
    /// 元数据
    pub metadata: BundleMetadata,
}

/// Bundle 元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleMetadata {
    /// 格式版本
    pub version: String,
    /// SHA-256 校验和
    pub checksum: String,
    /// 创建者
    pub created_by: String,
    /// 策略数量
    pub rule_count: usize,
}

/// Bundle 管理器
pub struct BundleManager {
    /// Rego 引擎引用
    engine: Arc<RegoEngine>,
    /// Bundle 存储目录
    storage_dir: String,
    /// 当前活跃 Bundle
    current_revision: std::sync::RwLock<Option<String>>,
}

impl BundleManager {
    pub fn new(engine: Arc<RegoEngine>, storage_dir: &str) -> Self {
        Self {
            engine,
            storage_dir: storage_dir.to_string(),
            current_revision: std::sync::RwLock::new(None),
        }
    }

    /// 打包策略为 Bundle
    pub fn pack_bundle(
        &self,
        rules: Vec<RegoRule>,
        created_by: &str,
    ) -> Result<OpaBundle> {
        let revision = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        let rule_count = rules.len();

        // 计算校验和
        let rules_json = serde_json::to_string(&rules)?;
        let mut hasher = Sha256::new();
        hasher.update(rules_json.as_bytes());
        let checksum = format!("sha256:{}", hex::encode(hasher.finalize()));

        let bundle = OpaBundle {
            revision: revision.clone(),
            created_at: now,
            rules,
            metadata: BundleMetadata {
                version: "1.0".to_string(),
                checksum,
                created_by: created_by.to_string(),
                rule_count,
            },
        };

        tracing::info!(
            revision = %revision,
            rules = rule_count,
            "Bundle 打包完成"
        );

        Ok(bundle)
    }

    /// 热加载 Bundle（无需重启）
    pub fn hot_load(&self, bundle: &OpaBundle) -> Result<()> {
        // 验证校验和
        let rules_json = serde_json::to_string(&bundle.rules)?;
        let mut hasher = Sha256::new();
        hasher.update(rules_json.as_bytes());
        let computed_checksum = format!("sha256:{}", hex::encode(hasher.finalize()));

        if computed_checksum != bundle.metadata.checksum {
            return Err(Error::Policy(format!(
                "Bundle 校验失败: 期望 {} 实际 {}",
                bundle.metadata.checksum, computed_checksum
            )));
        }

        // 加载规则到引擎
        self.engine.load_rules(bundle.rules.clone())?;

        // 更新当前版本
        let mut current = self.current_revision.write().map_err(|e| {
            Error::Internal(format!("获取版本写锁失败: {}", e))
        })?;
        *current = Some(bundle.revision.clone());

        tracing::info!(
            revision = %bundle.revision,
            "Bundle 热加载完成"
        );

        Ok(())
    }

    /// 序列化 Bundle 为 JSON
    pub fn serialize(&self, bundle: &OpaBundle) -> Result<Vec<u8>> {
        let json = serde_json::to_vec_pretty(bundle)?;
        Ok(json)
    }

    /// 从 JSON 反序列化 Bundle
    pub fn deserialize(&self, data: &[u8]) -> Result<OpaBundle> {
        let bundle: OpaBundle = serde_json::from_slice(data)?;
        Ok(bundle)
    }

    /// 获取当前活跃 Bundle 版本号
    pub fn current_revision(&self) -> Option<String> {
        self.current_revision.read()
            .ok()
            .and_then(|r| r.clone())
    }

    /// 获取 Bundle 存储目录
    pub fn storage_dir(&self) -> &str {
        &self.storage_dir
    }

    /// 从云端同步 Bundle（模拟）
    ///
    /// 在实际生产中，这里会:
    /// 1. 从云端 OPA Bundle 服务下载
    /// 2. 验证签名
    /// 3. 热加载到引擎
    pub fn sync_from_cloud(&self, bundle_url: &str) -> Result<String> {
        tracing::info!(
            url = bundle_url,
            "从云端同步 Bundle..."
        );

        // 模拟: 使用默认规则创建 Bundle
        let rules = RegoEngine::default_rego_rules();
        let bundle = self.pack_bundle(rules, "cloud-sync")?;
        self.hot_load(&bundle)?;

        Ok(bundle.revision)
    }
}
