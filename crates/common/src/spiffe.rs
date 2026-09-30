//! SPIFFE ID 解析与构造
//!
//! SPIFFE ID 格式: `spiffe://<trust-domain>/<path>/<name>`
//! 示例: `spiffe://iot-ztna.acme.com/device/SN-A001-2026`

use crate::error::{Error, Result};
use std::fmt;

/// SPIFFE ID
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpiffeId {
    /// 信任域，如 `iot-ztna.acme.com`
    pub trust_domain: String,
    /// 路径类型，如 `device` / `gateway` / `service`
    pub path_type: SpiffePathType,
    /// 实体名称，如设备序列号
    pub name: String,
}

/// SPIFFE ID 路径类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpiffePathType {
    Device,
    Gateway,
    Service,
}

impl fmt::Display for SpiffePathType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpiffePathType::Device => write!(f, "device"),
            SpiffePathType::Gateway => write!(f, "gateway"),
            SpiffePathType::Service => write!(f, "service"),
        }
    }
}

impl SpiffeId {
    /// 从字符串解析 SPIFFE ID
    pub fn parse(s: &str) -> Result<Self> {
        let s = s.trim();
        if !s.starts_with("spiffe://") {
            return Err(Error::SpiffeIdFormat(format!(
                "SPIFFE ID 必须以 'spiffe://' 开头，实际: {}",
                s
            )));
        }
        let rest = &s["spiffe://".len()..];
        let parts: Vec<&str> = rest.splitn(3, '/').collect();
        if parts.len() < 3 {
            return Err(Error::SpiffeIdFormat(format!(
                "SPIFFE ID 路径不完整，期望 spiffe://<domain>/<type>/<name>，实际: {}",
                s
            )));
        }
        let trust_domain = parts[0].to_string();
        if trust_domain.is_empty() {
            return Err(Error::SpiffeIdFormat("信任域不能为空".to_string()));
        }
        let path_type = match parts[1] {
            "device" => SpiffePathType::Device,
            "gateway" => SpiffePathType::Gateway,
            "service" => SpiffePathType::Service,
            other => {
                return Err(Error::SpiffeIdFormat(format!(
                    "未知路径类型: {}，期望 device/gateway/service",
                    other
                )))
            }
        };
        let name = parts[2].to_string();
        if name.is_empty() {
            return Err(Error::SpiffeIdFormat("名称不能为空".to_string()));
        }
        Ok(Self {
            trust_domain,
            path_type,
            name,
        })
    }

    /// 构造设备 SPIFFE ID
    pub fn for_device(trust_domain: &str, serial: &str) -> Self {
        Self {
            trust_domain: trust_domain.to_string(),
            path_type: SpiffePathType::Device,
            name: serial.to_string(),
        }
    }

    /// 构造网关 SPIFFE ID
    pub fn for_gateway(trust_domain: &str, gateway_id: &str) -> Self {
        Self {
            trust_domain: trust_domain.to_string(),
            path_type: SpiffePathType::Gateway,
            name: gateway_id.to_string(),
        }
    }

    /// 构造服务 SPIFFE ID
    pub fn for_service(trust_domain: &str, service_name: &str) -> Self {
        Self {
            trust_domain: trust_domain.to_string(),
            path_type: SpiffePathType::Service,
            name: service_name.to_string(),
        }
    }

    /// 返回设备序列号（仅对 device 类型有效）
    pub fn serial_number(&self) -> Option<&str> {
        match self.path_type {
            SpiffePathType::Device => Some(&self.name),
            _ => None,
        }
    }
}

impl fmt::Display for SpiffeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "spiffe://{}/{}/{}", self.trust_domain, self.path_type, self.name)
    }
}

impl serde::Serialize for SpiffeId {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for SpiffeId {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        SpiffeId::parse(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_device_id() {
        let id = SpiffeId::parse("spiffe://iot-ztna.acme.com/device/SN-A001").unwrap();
        assert_eq!(id.trust_domain, "iot-ztna.acme.com");
        assert_eq!(id.path_type, SpiffePathType::Device);
        assert_eq!(id.name, "SN-A001");
        assert_eq!(id.serial_number(), Some("SN-A001"));
    }

    #[test]
    fn parse_gateway_id() {
        let id = SpiffeId::parse("spiffe://iot-ztna.acme.com/gateway/edge-gw-01").unwrap();
        assert_eq!(id.path_type, SpiffePathType::Gateway);
    }

    #[test]
    fn roundtrip_display() {
        let id = SpiffeId::for_device("iot-ztna.acme.com", "SN-TEST");
        let s = id.to_string();
        let parsed = SpiffeId::parse(&s).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn invalid_formats() {
        assert!(SpiffeId::parse("http://example.com").is_err());
        assert!(SpiffeId::parse("spiffe://only-domain").is_err());
        assert!(SpiffeId::parse("spiffe://domain/unknown/name").is_err());
        assert!(SpiffeId::parse("spiffe://domain/device/").is_err());
    }
}
