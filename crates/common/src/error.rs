//! 公共错误类型

use thiserror::Error;

/// IoT-ZTNA 统一错误类型
#[derive(Error, Debug)]
pub enum Error {
    #[error("配置错误: {0}")]
    Config(String),

    #[error("数据库错误: {0}")]
    Database(String),

    #[error("身份认证错误: {0}")]
    Auth(String),

    #[error("证书错误: {0}")]
    Certificate(String),

    #[error("策略错误: {0}")]
    Policy(String),

    #[error("SPIFFE ID 格式错误: {0}")]
    SpiffeIdFormat(String),

    #[error("设备未找到: {0}")]
    DeviceNotFound(String),

    #[error("设备已存在: {0}")]
    DeviceAlreadyExists(String),

    #[error("证书已过期: spiffe_id={spiffe_id}")]
    CertificateExpired { spiffe_id: String },

    #[error("证书已吊销: spiffe_id={spiffe_id}")]
    CertificateRevoked { spiffe_id: String },

    #[error("策略评估失败: {0}")]
    PolicyEvaluation(String),

    #[error("注册令牌无效或已过期")]
    InvalidRegistrationToken,

    #[error("硬件指纹不匹配: expected={expected}, actual={actual}")]
    HardwareFingerprintMismatch { expected: String, actual: String },

    #[error("内部错误: {0}")]
    Internal(String),

    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("序列化错误: {0}")]
    Serde(#[from] serde_json::Error),
}

impl Error {
    /// 返回 HTTP 状态码
    pub fn status_code(&self) -> u16 {
        match self {
            Error::DeviceNotFound(_) => 404,
            Error::DeviceAlreadyExists(_) => 409,
            Error::CertificateExpired { .. } => 403,
            Error::CertificateRevoked { .. } => 403,
            Error::InvalidRegistrationToken => 401,
            Error::HardwareFingerprintMismatch { .. } => 403,
            Error::Auth(_) => 401,
            _ => 500,
        }
    }

    /// 返回错误码字符串
    pub fn error_code(&self) -> &'static str {
        match self {
            Error::Config(_) => "CONFIG_ERROR",
            Error::Database(_) => "DATABASE_ERROR",
            Error::Auth(_) => "AUTH_ERROR",
            Error::Certificate(_) => "CERTIFICATE_ERROR",
            Error::Policy(_) => "POLICY_ERROR",
            Error::SpiffeIdFormat(_) => "SPIFFE_ID_FORMAT_ERROR",
            Error::DeviceNotFound(_) => "DEVICE_NOT_FOUND",
            Error::DeviceAlreadyExists(_) => "DEVICE_ALREADY_EXISTS",
            Error::CertificateExpired { .. } => "CERTIFICATE_EXPIRED",
            Error::CertificateRevoked { .. } => "CERTIFICATE_REVOKED",
            Error::PolicyEvaluation(_) => "POLICY_EVALUATION_ERROR",
            Error::InvalidRegistrationToken => "INVALID_REGISTRATION_TOKEN",
            Error::HardwareFingerprintMismatch { .. } => "HARDWARE_FINGERPRINT_MISMATCH",
            Error::Internal(_) => "INTERNAL_ERROR",
            Error::Io(_) => "IO_ERROR",
            Error::Serde(_) => "SERDE_ERROR",
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Error::Database(e.to_string())
    }
}

impl From<ring::error::Unspecified> for Error {
    fn from(e: ring::error::Unspecified) -> Self {
        Error::Certificate(format!("密码学操作失败: {:?}", e))
    }
}


