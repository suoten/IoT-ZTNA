//! IoT-ZTNA 公共类型与工具
//!
//! 包含错误类型、SPIFFE ID 解析、设备分级、通用数据结构。

pub mod error;
pub mod spiffe;
pub mod types;
pub mod config;
pub mod behavior;
pub mod crypto;

pub use error::{Error, Result};
pub use spiffe::SpiffeId;
pub use types::*;
pub use config::GatewayConfig;
pub use behavior::BehaviorEngine;
pub use crypto::{ConfigEncryptor, EncryptionKey};
