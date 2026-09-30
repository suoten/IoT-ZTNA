//! 证书吊销服务（含 CRL 生成）

use iot_ztna_common::{DeviceStatus, Result, SpiffeId};
use iot_ztna_storage::Database;

/// 吊销服务
pub struct RevocationService {
    db: Database,
}

impl RevocationService {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// 吊销设备（同时吊销其所有证书）
    pub fn revoke_device(&self, spiffe_id: &SpiffeId, reason: &str) -> Result<()> {
        // 吊销所有活跃证书
        self.db.revoke_certificates_by_spiffe(spiffe_id)?;

        // 更新设备状态
        self.db.update_device_status(spiffe_id, &DeviceStatus::Revoked)?;

        // 审计
        self.db.audit(
            "admin",
            "revoke_device",
            &spiffe_id.to_string(),
            "success",
            serde_json::json!({ "reason": reason }),
            "",
        )?;

        tracing::info!(
            spiffe_id = %spiffe_id,
            reason = reason,
            "设备已吊销"
        );

        Ok(())
    }

    /// 恢复设备
    pub fn reactivate_device(&self, spiffe_id: &SpiffeId) -> Result<()> {
        self.db
            .update_device_status(spiffe_id, &DeviceStatus::Offline)?;
        self.db.audit(
            "admin",
            "reactivate_device",
            &spiffe_id.to_string(),
            "success",
            serde_json::json!({}),
            "",
        )?;
        Ok(())
    }

    /// 检查设备是否已吊销
    pub fn is_revoked(&self, spiffe_id: &SpiffeId) -> Result<bool> {
        let device = self.db.get_device_by_spiffe(spiffe_id)?;
        Ok(device.map(|d| d.status == DeviceStatus::Revoked).unwrap_or(true))
    }

    /// 注销设备（删除设备 + 所有证书）
    pub fn delete_device(&self, spiffe_id: &SpiffeId) -> Result<()> {
        self.db.revoke_certificates_by_spiffe(spiffe_id)?;
        self.db.delete_device(spiffe_id)?;
        self.db.audit(
            "admin",
            "delete_device",
            &spiffe_id.to_string(),
            "success",
            serde_json::json!({}),
            "",
        )?;
        Ok(())
    }

    /// 生成 CRL（证书吊销列表）JSON 格式
    ///
    /// 返回所有已吊销证书的列表，可用于 mTLS 握手时检查。
    pub fn generate_crl(&self) -> Result<serde_json::Value> {
        let revoked = self.db.list_revoked_certificates()?;
        Ok(serde_json::json!({
            "version": 1,
            "generated_at": chrono::Utc::now().to_rfc3339(),
            "revoked_certificates": revoked.iter().map(|c| {
                serde_json::json!({
                    "serial_number": c.serial_number,
                    "spiffe_id": c.spiffe_id,
                    "revoked_at": c.revoked_at,
                    "expires_at": c.expires_at,
                })
            }).collect::<Vec<_>>(),
            "total_revoked": revoked.len(),
        }))
    }

    /// 检查证书序列号是否已被吊销（OCSP-like 本地检查）
    pub fn is_cert_revoked(&self, serial_number: &str) -> Result<bool> {
        let revoked = self.db.list_revoked_certificates()?;
        Ok(revoked.iter().any(|c| c.serial_number == serial_number))
    }
}
