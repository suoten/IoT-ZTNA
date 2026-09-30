//! SVID 管理器
//!
//! 负责设备 SVID 证书的签发、存储、续期、查询。

use crate::CertificateAuthority;
use chrono::{Duration, Utc};
use iot_ztna_common::{
    Certificate, CertificateStatus, DeviceStatus, Error, Result, SpiffeId, SvidResponse,
};
use iot_ztna_storage::Database;
use uuid::Uuid;

/// SVID 管理器
pub struct SvidManager {
    db: Database,
    ca: CertificateAuthority,
    svid_ttl_days: u32,
    renewal_trigger_hours: u32,
}

impl SvidManager {
    pub fn new(
        db: Database,
        ca: CertificateAuthority,
        svid_ttl_days: u32,
        renewal_trigger_hours: u32,
    ) -> Self {
        Self {
            db,
            ca,
            svid_ttl_days,
            renewal_trigger_hours,
        }
    }

    /// 为设备签发 SVID 证书
    pub fn issue_svid(&self, spiffe_id: &SpiffeId) -> Result<SvidResponse> {
        // 检查设备是否存在
        let device = self
            .db
            .get_device_by_spiffe(spiffe_id)?
            .ok_or_else(|| Error::DeviceNotFound(spiffe_id.to_string()))?;

        if device.status == DeviceStatus::Revoked {
            return Err(Error::CertificateRevoked {
                spiffe_id: spiffe_id.to_string(),
            });
        }

        // 签发证书
        let (cert_pem, _key_pem, expires_at) = self.ca.issue_svid(spiffe_id, self.svid_ttl_days)?;

        // 存储证书
        let cert = Certificate {
            id: Uuid::new_v4().to_string(),
            spiffe_id: spiffe_id.clone(),
            serial_number: Uuid::new_v4().to_string(),
            pem: cert_pem.clone(),
            issued_at: Utc::now(),
            expires_at,
            status: CertificateStatus::Active,
        };
        self.db.insert_certificate(&cert)?;

        // 更新设备状态为在线
        self.db.update_device_status(spiffe_id, &DeviceStatus::Online)?;

        tracing::info!(
            spiffe_id = %spiffe_id,
            expires_at = %expires_at,
            "SVID 签发成功"
        );

        Ok(SvidResponse {
            spiffe_id: spiffe_id.clone(),
            svid: cert_pem,
            bundle: self.ca.trust_bundle_pem(),
            expires_at,
        })
    }

    /// 检查设备证书是否需要续期
    pub fn check_renewal(&self, spiffe_id: &SpiffeId) -> Result<bool> {
        let cert = match self.db.get_active_certificate(spiffe_id)? {
            Some(c) => c,
            None => return Ok(true), // 无活跃证书，需要签发
        };

        let now = Utc::now();
        let renewal_threshold =
            cert.expires_at - Duration::hours(self.renewal_trigger_hours as i64);
        Ok(now > renewal_threshold)
    }

    /// 自动续期检查（对即将过期的证书自动续期）
    pub fn auto_renew(&self) -> Result<Vec<SvidResponse>> {
        let expiring = self.db.get_expiring_certificates(self.renewal_trigger_hours / 24 + 1)?;
        let mut renewed = Vec::new();
        for cert in expiring {
            if cert.status != CertificateStatus::Active {
                continue;
            }
            // 旧证书标记过期
            self.db
                .update_certificate_status(&cert.id, &CertificateStatus::Expired)?;
            // 签发新证书
            match self.issue_svid(&cert.spiffe_id) {
                Ok(resp) => {
                    tracing::info!(
                        spiffe_id = %cert.spiffe_id,
                        "证书自动续期成功"
                    );
                    renewed.push(resp);
                }
                Err(e) => {
                    tracing::error!(
                        spiffe_id = %cert.spiffe_id,
                        error = %e,
                        "证书续期失败"
                    );
                }
            }
        }
        Ok(renewed)
    }

    /// 手动续期
    pub fn renew_svid(&self, spiffe_id: &SpiffeId) -> Result<SvidResponse> {
        // 吊销旧证书
        self.db.revoke_certificates_by_spiffe(spiffe_id)?;
        // 签发新证书
        self.issue_svid(spiffe_id)
    }

    /// 获取证书剩余天数
    pub fn certificate_expiry_days(&self, spiffe_id: &SpiffeId) -> Result<Option<i64>> {
        let cert = self.db.get_active_certificate(spiffe_id)?;
        Ok(cert.map(|c| (c.expires_at - Utc::now()).num_days()))
    }

    /// 清理过期证书标记
    pub fn cleanup_expired(&self) -> Result<u64> {
        self.db.mark_expired_certificates()
    }
}
