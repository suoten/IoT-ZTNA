//! 设备注册服务
//!
//! 支持 OOB 带外注册、JIT 即时注册、批量注册。

use crate::SvidManager;
use chrono::{Duration, Utc};
use iot_ztna_common::{
    Device, DeviceRegistrationRequest, DeviceStatus, Error, Result, SpiffeId,
    SvidResponse, TrustLevel, TrustRoot,
};
use iot_ztna_storage::Database;
use uuid::Uuid;

/// 注册服务
pub struct RegistrationService {
    db: Database,
    trust_domain: String,
}

/// 注册令牌
#[derive(Debug, Clone)]
pub struct RegistrationToken {
    pub token: String,
    pub serial_number: String,
    pub hardware_fingerprint: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

impl RegistrationService {
    pub fn new(db: Database, trust_domain: &str) -> Self {
        Self {
            db,
            trust_domain: trust_domain.to_string(),
        }
    }

    /// 生成注册令牌（OOB 模式）
    pub fn generate_token(
        &self,
        serial_number: &str,
        hardware_fingerprint: &str,
        ttl_hours: u32,
    ) -> Result<RegistrationToken> {
        let token = format!("reg_{}", Uuid::new_v4().simple());
        let expires_at = Utc::now() + Duration::hours(ttl_hours as i64);

        let conn = self.db.conn()?;
        conn.execute(
            r#"INSERT INTO registration_tokens
               (id, token, serial_number, hardware_fingerprint, expires_at, used, created_at)
               VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6)"#,
            rusqlite::params![
                Uuid::new_v4().to_string(),
                &token,
                serial_number,
                hardware_fingerprint,
                expires_at.to_rfc3339(),
                Utc::now().to_rfc3339(),
            ],
        )
        .map_err(|e| Error::Database(format!("存储注册令牌失败: {}", e)))?;

        tracing::info!(serial_number = serial_number, "生成注册令牌");
        Ok(RegistrationToken {
            token,
            serial_number: serial_number.to_string(),
            hardware_fingerprint: hardware_fingerprint.to_string(),
            expires_at,
        })
    }

    /// 验证注册令牌
    pub fn validate_token(&self, token: &str) -> Result<RegistrationToken> {
        let conn = self.db.conn()?;
        let row = conn
            .query_row(
                "SELECT token, serial_number, hardware_fingerprint, expires_at, used FROM registration_tokens WHERE token = ?1",
                rusqlite::params![token],
                |row| {
                    Ok(RegistrationToken {
                        token: row.get(0)?,
                        serial_number: row.get(1)?,
                        hardware_fingerprint: row.get(2)?,
                        expires_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                    })
                },
            )
            .ok();

        match row {
            Some(t) => {
                if t.expires_at < Utc::now() {
                    return Err(Error::InvalidRegistrationToken);
                }
                // 标记为已使用
                conn.execute(
                    "UPDATE registration_tokens SET used = 1 WHERE token = ?1",
                    rusqlite::params![token],
                )
                .map_err(|e| Error::Database(format!("更新令牌状态失败: {}", e)))?;
                Ok(t)
            }
            None => Err(Error::InvalidRegistrationToken),
        }
    }

    /// 注册设备（OOB 模式）
    pub fn register_device(
        &self,
        req: DeviceRegistrationRequest,
        svid_manager: &SvidManager,
    ) -> Result<(Device, SvidResponse)> {
        // 检查设备是否已存在
        if let Some(_existing) = self.db.get_device_by_serial(&req.serial_number)? {
            return Err(Error::DeviceAlreadyExists(req.serial_number));
        }

        // 创建设备记录
        let spiffe_id = SpiffeId::for_device(&self.trust_domain, &req.serial_number);
        let device = Device {
            id: Uuid::new_v4().to_string(),
            spiffe_id: spiffe_id.clone(),
            serial_number: req.serial_number.clone(),
            model: req.model,
            trust_root: req.trust_root,
            trust_level: req.trust_level,
            status: DeviceStatus::Pending,
            capabilities: req.capabilities,
            registered_at: Utc::now(),
            last_seen: None,
            maintenance_mode: false,
        };

        self.db.insert_device(&device)?;

        // 签发首个 SVID
        let svid = svid_manager.issue_svid(&spiffe_id)?;

        // 审计日志
        self.db.audit(
            "system",
            "register_device",
            &spiffe_id.to_string(),
            "success",
            serde_json::json!({
                "serial_number": device.serial_number,
                "model": device.model,
                "trust_level": device.trust_level.to_string(),
            }),
            "",
        )?;

        tracing::info!(
            serial_number = %device.serial_number,
            spiffe_id = %spiffe_id,
            "设备注册成功"
        );

        Ok((device, svid))
    }

    /// 批量注册（工厂场景）
    pub fn batch_register(
        &self,
        requests: Vec<DeviceRegistrationRequest>,
        svid_manager: &SvidManager,
    ) -> Result<Vec<Result<(Device, SvidResponse)>>> {
        let mut results = Vec::new();
        for req in requests {
            results.push(self.register_device(req, svid_manager));
        }
        Ok(results)
    }

    /// JIT 即时注册（设备携带出厂证书）
    pub fn jit_register(
        &self,
        serial_number: &str,
        model: &str,
        capabilities: Vec<String>,
        trust_level: TrustLevel,
        svid_manager: &SvidManager,
    ) -> Result<(Device, SvidResponse)> {
        let req = DeviceRegistrationRequest {
            serial_number: serial_number.to_string(),
            model: model.to_string(),
            trust_root: TrustRoot::FLASH,
            trust_level,
            capabilities,
            hardware_fingerprint: String::new(),
        };
        self.register_device(req, svid_manager)
    }
}
