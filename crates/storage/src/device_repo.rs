//! 设备仓储

use crate::Database;
use chrono::Utc;
use iot_ztna_common::{
    Device, DeviceStatus, Error, Result, SpiffeId, TrustLevel, TrustRoot,
};
use rusqlite::params;

impl Database {
    /// 注册设备
    pub fn insert_device(&self, device: &Device) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            r#"INSERT INTO devices
               (id, spiffe_id, serial_number, model, trust_root, trust_level,
                status, capabilities, hardware_fingerprint, registered_at, last_seen, maintenance_mode)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"#,
            params![
                device.id,
                device.spiffe_id.to_string(),
                device.serial_number,
                device.model,
                device.trust_root.to_string(),
                device.trust_level.to_string(),
                device.status.to_string(),
                serde_json::to_string(&device.capabilities).unwrap_or_default(),
                "",
                device.registered_at.to_rfc3339(),
                device.last_seen.map(|t| t.to_rfc3339()),
                device.maintenance_mode as i32,
            ],
        )
        .map_err(|e| Error::Database(format!("插入设备失败: {}", e)))?;
        Ok(())
    }

    /// 按 SPIFFE ID 查询设备
    pub fn get_device_by_spiffe(&self, spiffe_id: &SpiffeId) -> Result<Option<Device>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare(
                r#"SELECT id, spiffe_id, serial_number, model, trust_root, trust_level,
                          status, capabilities, registered_at, last_seen, maintenance_mode
                   FROM devices WHERE spiffe_id = ?1"#,
            )
            .map_err(|e| Error::Database(format!("查询设备准备失败: {}", e)))?;
        let device = stmt
            .query_row(params![spiffe_id.to_string()], |row| {
                Ok(Device {
                    id: row.get(0)?,
                    spiffe_id: SpiffeId::parse(&row.get::<_, String>(1)?).unwrap_or_else(|_| spiffe_id.clone()),
                    serial_number: row.get(2)?,
                    model: row.get(3)?,
                    trust_root: row.get::<_, String>(4).and_then(|s| s.parse().map_err(|_| rusqlite::Error::InvalidQuery)).unwrap_or(TrustRoot::FLASH),
                    trust_level: match row.get::<_, String>(5)?.as_str() {
                        "A" => TrustLevel::A,
                        "B" => TrustLevel::B,
                        _ => TrustLevel::C,
                    },
                    status: match row.get::<_, String>(6)?.as_str() {
                        "online" => DeviceStatus::Online,
                        "offline" => DeviceStatus::Offline,
                        "revoked" => DeviceStatus::Revoked,
                        _ => DeviceStatus::Pending,
                    },
                    capabilities: serde_json::from_str(&row.get::<_, String>(7).unwrap_or_default()).unwrap_or_default(),
                    registered_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?).map(|dt| dt.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
                    last_seen: row.get::<_, Option<String>>(9)?.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok()).map(|dt| dt.with_timezone(&Utc)),
                    maintenance_mode: row.get::<_, i32>(10)? != 0,
                })
            })
            .ok();
        Ok(device)
    }

    /// 按序列号查询设备
    pub fn get_device_by_serial(&self, serial: &str) -> Result<Option<Device>> {
        let conn = self.conn()?;
        let spiffe_id_str: Option<String> = conn
            .query_row(
                "SELECT spiffe_id FROM devices WHERE serial_number = ?1",
                params![serial],
                |row| row.get(0),
            )
            .ok();
        match spiffe_id_str {
            Some(sid) => {
                let spiffe_id = SpiffeId::parse(&sid)?;
                self.get_device_by_spiffe(&spiffe_id)
            }
            None => Ok(None),
        }
    }

    /// 列出所有设备
    pub fn list_devices(&self, limit: u32, offset: u32) -> Result<Vec<Device>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare(
                r#"SELECT id, spiffe_id, serial_number, model, trust_root, trust_level,
                          status, capabilities, registered_at, last_seen, maintenance_mode
                   FROM devices ORDER BY registered_at DESC LIMIT ?1 OFFSET ?2"#,
            )
            .map_err(|e| Error::Database(format!("准备设备查询失败: {}", e)))?;
        let devices = stmt
            .query_map(params![limit as i64, offset as i64], |row| {
                Ok(Device {
                    id: row.get(0)?,
                    spiffe_id: SpiffeId::parse(&row.get::<_, String>(1)?).unwrap_or_else(|_| SpiffeId::for_device("unknown", "unknown")),
                    serial_number: row.get(2)?,
                    model: row.get(3)?,
                    trust_root: row.get::<_, String>(4).and_then(|s| s.parse().map_err(|_| rusqlite::Error::InvalidQuery)).unwrap_or(TrustRoot::FLASH),
                    trust_level: match row.get::<_, String>(5)?.as_str() {
                        "A" => TrustLevel::A,
                        "B" => TrustLevel::B,
                        _ => TrustLevel::C,
                    },
                    status: match row.get::<_, String>(6)?.as_str() {
                        "online" => DeviceStatus::Online,
                        "offline" => DeviceStatus::Offline,
                        "revoked" => DeviceStatus::Revoked,
                        _ => DeviceStatus::Pending,
                    },
                    capabilities: serde_json::from_str(&row.get::<_, String>(7).unwrap_or_default()).unwrap_or_default(),
                    registered_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?).map(|dt| dt.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
                    last_seen: row.get::<_, Option<String>>(9)?.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok()).map(|dt| dt.with_timezone(&Utc)),
                    maintenance_mode: row.get::<_, i32>(10)? != 0,
                })
            })
            .map_err(|e| Error::Database(format!("查询设备列表失败: {}", e)))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(devices)
    }

    /// 统计设备总数
    pub fn count_devices(&self) -> Result<u64> {
        let conn = self.conn()?;
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM devices", [], |row| row.get(0))
            .unwrap_or(0);
        Ok(count as u64)
    }

    /// 统计在线设备
    pub fn count_online_devices(&self) -> Result<u64> {
        let conn = self.conn()?;
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM devices WHERE status = 'online'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        Ok(count as u64)
    }

    /// 更新设备状态
    pub fn update_device_status(&self, spiffe_id: &SpiffeId, status: &DeviceStatus) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "UPDATE devices SET status = ?1, last_seen = ?2 WHERE spiffe_id = ?3",
            params![status.to_string(), Utc::now().to_rfc3339(), spiffe_id.to_string()],
        )
        .map_err(|e| Error::Database(format!("更新设备状态失败: {}", e)))?;
        Ok(())
    }

    /// 更新最后在线时间
    pub fn touch_device(&self, spiffe_id: &SpiffeId) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "UPDATE devices SET last_seen = ?1 WHERE spiffe_id = ?2",
            params![Utc::now().to_rfc3339(), spiffe_id.to_string()],
        )
        .map_err(|e| Error::Database(format!("更新在线时间失败: {}", e)))?;
        Ok(())
    }

    /// 设置维护模式
    pub fn set_maintenance_mode(&self, spiffe_id: &SpiffeId, enabled: bool) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "UPDATE devices SET maintenance_mode = ?1 WHERE spiffe_id = ?2",
            params![enabled as i32, spiffe_id.to_string()],
        )
        .map_err(|e| Error::Database(format!("更新维护模式失败: {}", e)))?;
        Ok(())
    }

    /// 删除设备
    pub fn delete_device(&self, spiffe_id: &SpiffeId) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "DELETE FROM devices WHERE spiffe_id = ?1",
            params![spiffe_id.to_string()],
        )
        .map_err(|e| Error::Database(format!("删除设备失败: {}", e)))?;
        Ok(())
    }
}
