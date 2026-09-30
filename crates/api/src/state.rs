//! 应用共享状态

use iot_ztna_common::{BehaviorEngine, GatewayConfig};
use iot_ztna_identity::{RegistrationService, RevocationService, SvidManager};
use iot_ztna_policy::PolicyManager;
use iot_ztna_storage::Database;
use iot_ztna_dataplane::DataPlaneManager;
use iot_ztna_ai::{AutoencoderEngine, ModelManager};
use iot_ztna_opa::{RegoEngine, BundleManager};
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::auth::AuthConfig;

/// 应用状态
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<GatewayConfig>,
    pub db: Database,
    pub svid_manager: Arc<SvidManager>,
    pub registration: Arc<RegistrationService>,
    pub revocation: Arc<RevocationService>,
    pub policy_manager: Arc<PolicyManager>,
    /// 告警广播通道
    pub alert_tx: broadcast::Sender<String>,
    /// 设备事件广播通道
    pub device_tx: broadcast::Sender<String>,
    /// 指标广播通道
    pub metrics_tx: broadcast::Sender<String>,
    /// 行为检测引擎（Z-Score 统计模型）
    pub behavior_engine: Arc<BehaviorEngine>,
    /// Autoencoder 行为基线检测引擎
    pub ae_engine: Arc<AutoencoderEngine>,
    /// ONNX 模型管理器
    pub model_manager: Arc<ModelManager>,
    /// 数据面管理器（eBPF/nftables）
    pub dataplane: Arc<DataPlaneManager>,
    /// Rego 策略引擎
    pub rego_engine: Arc<RegoEngine>,
    /// OPA Bundle 管理器
    pub bundle_manager: Arc<BundleManager>,
    /// JWT 认证配置
    pub auth_config: Arc<AuthConfig>,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("config", &self.config.gateway_id)
            .finish()
    }
}
