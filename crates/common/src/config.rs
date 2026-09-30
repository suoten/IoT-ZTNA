//! 网关配置管理

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// 网关配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    /// 信任域
    pub trust_domain: String,
    /// 网关 ID
    pub gateway_id: String,
    /// HTTP 监听地址
    pub listen_addr: String,
    /// HTTP 监听端口
    pub listen_port: u16,
    /// TLS 配置
    pub tls: TlsConfig,
    /// 数据库路径
    pub db_path: String,
    /// 证书默认有效期（天）
    pub svid_ttl_days: u32,
    /// 证书续期触发时间（到期前小时数）
    pub renewal_trigger_hours: u32,
    /// 吊销检查间隔（秒）
    pub revocation_check_interval_secs: u64,
    /// Prometheus 指标端点
    pub metrics_endpoint: String,
    /// 日志级别
    pub log_level: String,
    /// 离线模式
    pub offline_mode: bool,
    /// nftables 表名
    pub nft_table: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// 证书文件路径
    pub cert_path: String,
    /// 私钥文件路径
    pub key_path: String,
    /// CA 证书路径
    pub ca_cert_path: String,
    /// 是否启用 mTLS
    pub mtls_enabled: bool,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            trust_domain: "iot-ztna.local".to_string(),
            gateway_id: "edge-gw-01".to_string(),
            listen_addr: "0.0.0.0".to_string(),
            listen_port: 8443,
            tls: TlsConfig {
                cert_path: "data/certs/gateway.crt".to_string(),
                key_path: "data/certs/gateway.key".to_string(),
                ca_cert_path: "data/certs/ca.crt".to_string(),
                mtls_enabled: true,
            },
            db_path: "data/ztna.db".to_string(),
            svid_ttl_days: 7,
            renewal_trigger_hours: 24,
            revocation_check_interval_secs: 300,
            metrics_endpoint: "/metrics".to_string(),
            log_level: "info".to_string(),
            offline_mode: false,
            nft_table: "iot_ztna".to_string(),
        }
    }
}

impl GatewayConfig {
    /// 从 TOML 文件加载配置
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| Error::Config(format!("读取配置文件失败 {}: {}", path.display(), e)))?;
        Self::from_toml_str(&content)
    }

    /// 从 TOML 字符串加载
    pub fn from_toml_str(s: &str) -> Result<Self> {
        // 简易 TOML 解析，不依赖 toml crate
        let mut config = GatewayConfig::default();
        let mut current_section = String::new();
        for line in s.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') {
                current_section = line[1..line.len() - 1].to_string();
                continue;
            }
            if let Some((key, val)) = line.split_once('=') {
                let key = key.trim();
                let val = val.trim().trim_matches('"');
                match (current_section.as_str(), key) {
                    ("", "trust_domain") => config.trust_domain = val.to_string(),
                    ("", "gateway_id") => config.gateway_id = val.to_string(),
                    ("", "listen_addr") => config.listen_addr = val.to_string(),
                    ("", "listen_port") => {
                        config.listen_port = val.parse().unwrap_or(8443)
                    }
                    ("", "db_path") => config.db_path = val.to_string(),
                    ("", "svid_ttl_days") => {
                        config.svid_ttl_days = val.parse().unwrap_or(7)
                    }
                    ("", "log_level") => config.log_level = val.to_string(),
                    ("", "offline_mode") => {
                        config.offline_mode = val == "true"
                    }
                    ("", "nft_table") => config.nft_table = val.to_string(),
                    ("tls", "cert_path") => config.tls.cert_path = val.to_string(),
                    ("tls", "key_path") => config.tls.key_path = val.to_string(),
                    ("tls", "ca_cert_path") => config.tls.ca_cert_path = val.to_string(),
                    ("tls", "mtls_enabled") => {
                        config.tls.mtls_enabled = val == "true"
                    }
                    _ => {}
                }
            }
        }
        Ok(config)
    }

    /// 保存为 TOML 文件
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let content = self.to_toml_string();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                Error::Config(format!("创建目录失败: {}", e))
            })?;
        }
        std::fs::write(path, content)
            .map_err(|e| Error::Config(format!("写入配置文件失败: {}", e)))?;
        Ok(())
    }

    /// 转为 TOML 字符串
    pub fn to_toml_string(&self) -> String {
        format!(
            r#"# IoT-ZTNA 网关配置文件
# 生成时间: {timestamp}

trust_domain = "{trust_domain}"
gateway_id = "{gateway_id}"
listen_addr = "{listen_addr}"
listen_port = {listen_port}
db_path = "{db_path}"
svid_ttl_days = {svid_ttl_days}
renewal_trigger_hours = {renewal_trigger_hours}
revocation_check_interval_secs = {revocation_check_interval_secs}
log_level = "{log_level}"
offline_mode = {offline_mode}
nft_table = "{nft_table}"

[tls]
cert_path = "{cert_path}"
key_path = "{key_path}"
ca_cert_path = "{ca_cert_path}"
mtls_enabled = {mtls_enabled}
"#,
            timestamp = chrono::Utc::now().to_rfc3339(),
            trust_domain = self.trust_domain,
            gateway_id = self.gateway_id,
            listen_addr = self.listen_addr,
            listen_port = self.listen_port,
            db_path = self.db_path,
            svid_ttl_days = self.svid_ttl_days,
            renewal_trigger_hours = self.renewal_trigger_hours,
            revocation_check_interval_secs = self.revocation_check_interval_secs,
            log_level = self.log_level,
            offline_mode = self.offline_mode,
            nft_table = self.nft_table,
            cert_path = self.tls.cert_path,
            key_path = self.tls.key_path,
            ca_cert_path = self.tls.ca_cert_path,
            mtls_enabled = self.tls.mtls_enabled,
        )
    }
}
