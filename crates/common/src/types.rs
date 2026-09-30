//! 公共数据类型

use crate::spiffe::SpiffeId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 设备信任等级
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TrustLevel {
    /// A 级: ARM Linux，完整 SPIRE Agent
    A,
    /// B 级: Cortex-M，轻量 mTLS SDK
    B,
    /// C 级: ESP32/受限设备，预置证书
    C,
}

impl std::fmt::Display for TrustLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrustLevel::A => write!(f, "A"),
            TrustLevel::B => write!(f, "B"),
            TrustLevel::C => write!(f, "C"),
        }
    }
}

/// 信任根类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TrustRoot {
    /// TPM 2.0
    TPM,
    /// 安全元件 Secure Element
    SE,
    /// Flash 证书烧录
    FLASH,
}

impl std::str::FromStr for TrustRoot {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "TPM" => Ok(TrustRoot::TPM),
            "SE" => Ok(TrustRoot::SE),
            "FLASH" => Ok(TrustRoot::FLASH),
            _ => Err(format!("未知信任根类型: {}", s)),
        }
    }
}

impl std::fmt::Display for TrustRoot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrustRoot::TPM => write!(f, "TPM"),
            TrustRoot::SE => write!(f, "SE"),
            TrustRoot::FLASH => write!(f, "FLASH"),
        }
    }
}

/// 设备状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceStatus {
    /// 在线
    Online,
    /// 离线
    Offline,
    /// 已吊销
    Revoked,
    /// 待注册
    Pending,
}

impl std::fmt::Display for DeviceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceStatus::Online => write!(f, "online"),
            DeviceStatus::Offline => write!(f, "offline"),
            DeviceStatus::Revoked => write!(f, "revoked"),
            DeviceStatus::Pending => write!(f, "pending"),
        }
    }
}

/// 设备信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: String,
    pub spiffe_id: SpiffeId,
    pub serial_number: String,
    pub model: String,
    pub trust_root: TrustRoot,
    pub trust_level: TrustLevel,
    pub status: DeviceStatus,
    pub capabilities: Vec<String>,
    pub registered_at: DateTime<Utc>,
    pub last_seen: Option<DateTime<Utc>>,
    pub maintenance_mode: bool,
}

/// 证书状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CertificateStatus {
    Active,
    Revoked,
    Expired,
}

impl std::fmt::Display for CertificateStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CertificateStatus::Active => write!(f, "active"),
            CertificateStatus::Revoked => write!(f, "revoked"),
            CertificateStatus::Expired => write!(f, "expired"),
        }
    }
}

/// 证书信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Certificate {
    pub id: String,
    pub spiffe_id: SpiffeId,
    pub serial_number: String,
    pub pem: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub status: CertificateStatus,
}

/// SVID 签发响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SvidResponse {
    pub spiffe_id: SpiffeId,
    pub svid: String,
    pub bundle: String,
    pub expires_at: DateTime<Utc>,
}

/// 策略状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PolicyStatus {
    Draft,
    Published,
    Archived,
}

impl std::fmt::Display for PolicyStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PolicyStatus::Draft => write!(f, "draft"),
            PolicyStatus::Published => write!(f, "published"),
            PolicyStatus::Archived => write!(f, "archived"),
        }
    }
}

/// 策略信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub id: String,
    pub version: String,
    pub name: String,
    pub rego_content: String,
    pub status: PolicyStatus,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
}

/// 策略决策结果
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PolicyDecision {
    Allow,
    Deny,
    Redirect,
    Log,
}

impl std::fmt::Display for PolicyDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PolicyDecision::Allow => write!(f, "allow"),
            PolicyDecision::Deny => write!(f, "deny"),
            PolicyDecision::Redirect => write!(f, "redirect"),
            PolicyDecision::Log => write!(f, "log"),
        }
    }
}

/// 策略决策请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRequest {
    pub device_spiffe_id: SpiffeId,
    pub protocol: String,
    pub operation: String,
    pub target: String,
    pub function_code: Option<u8>,
    pub topic: Option<String>,
    pub behavior_score: Option<f64>,
    pub maintenance_mode: bool,
}

/// 策略决策响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyResponse {
    pub decision: PolicyDecision,
    pub reason: String,
    pub matched_rules: Vec<String>,
    pub timestamp: DateTime<Utc>,
}

/// 告警级别
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

impl std::fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertSeverity::Info => write!(f, "info"),
            AlertSeverity::Warning => write!(f, "warning"),
            AlertSeverity::Critical => write!(f, "critical"),
        }
    }
}

/// 告警类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertType {
    DeviceOffline,
    CertificateExpiring,
    CertificateExpired,
    DeviceRevoked,
    PolicyDenied,
    BehaviorAnomaly,
    UnauthorizedDevice,
    SystemError,
}

/// 告警信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    pub spiffe_id: Option<SpiffeId>,
    pub message: String,
    pub details: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub acknowledged: bool,
}

/// 审计日志
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub actor: String,
    pub action: String,
    pub target: String,
    pub result: String,
    pub details: serde_json::Value,
    pub ip_address: String,
}

/// 设备注册请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRegistrationRequest {
    pub serial_number: String,
    pub model: String,
    pub trust_root: TrustRoot,
    pub trust_level: TrustLevel,
    pub capabilities: Vec<String>,
    pub hardware_fingerprint: String,
}

/// 统一 API 响应包装
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ApiError>,
    pub timestamp: DateTime<Utc>,
}

/// API 错误信息
#[derive(Debug, Serialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: Utc::now(),
        }
    }

    pub fn err(code: &str, message: &str) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ApiError {
                code: code.to_string(),
                message: message.to_string(),
            }),
            timestamp: Utc::now(),
        }
    }
}

/// 分页请求
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

/// 分页响应
#[derive(Debug, Serialize)]
pub struct PagedResponse<T: Serialize> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}

// === 行为检测相关类型 ===

/// 设备行为事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorEvent {
    /// 设备 SPIFFE ID
    pub spiffe_id: SpiffeId,
    /// 协议 (modbus / mqtt / opcua / http)
    pub protocol: String,
    /// 操作类型 (read / write / publish / subscribe)
    pub operation: String,
    /// 目标地址 (IP:port / topic / register)
    pub target: String,
    /// 功能码 (Modbus)
    pub function_code: Option<u8>,
    /// 数据大小 (bytes)
    pub data_size: Option<u32>,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
}

/// 行为基线统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorBaseline {
    pub spiffe_id: String,
    /// 各协议的请求频率（次/分钟）
    pub protocol_rates: serde_json::Value,
    /// 常用操作列表
    pub common_operations: Vec<String>,
    /// 常用目标列表
    pub common_targets: Vec<String>,
    /// 平均数据大小
    pub avg_data_size: f64,
    /// 活跃时间段（小时范围）
    pub active_hours: (u32, u32),
    /// 行为评分阈值
    pub anomaly_threshold: f64,
    /// 最后更新时间
    pub updated_at: DateTime<Utc>,
}

/// 行为检测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorAssessment {
    /// 异常评分 (0.0 - 1.0)
    pub anomaly_score: f64,
    /// 是否异常
    pub is_anomaly: bool,
    /// 检测到的异常类型
    pub anomaly_types: Vec<String>,
    /// 建议操作
    pub recommended_action: PolicyDecision,
    /// 评估原因
    pub reason: String,
    /// SHAP-style 特征贡献度（可解释性）
    /// 每个特征对异常评分的贡献分解
    pub feature_contributions: Vec<FeatureContribution>,
}

/// 特征贡献度（LIME/SHAP 风格可解释性）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureContribution {
    /// 特征名称
    pub feature: String,
    /// 贡献值（正值=推向异常，负值=推向正常）
    pub contribution: f64,
    /// 特征当前值
    pub value: String,
    /// 基线参考值
    pub baseline_value: String,
}

impl Default for BehaviorAssessment {
    fn default() -> Self {
        Self {
            anomaly_score: 0.0,
            is_anomaly: false,
            anomaly_types: vec![],
            recommended_action: PolicyDecision::Allow,
            reason: "正常行为".to_string(),
            feature_contributions: vec![],
        }
    }
}
