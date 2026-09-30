//! 证书仓储

use crate::Database;
use chrono::{DateTime, Utc};
use iot_ztna_common::{Certificate, CertificateStatus, Error, Result, SpiffeId};
use rusqlite::params;

impl Database {
    /// 插入证书
    pub fn insert_certificate(&self, cert: &Certificate) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            r#"INSERT INTO certificates
               (id, spiffe_id, serial_number, pem, issued_at, expires_at, status)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#,
            params![
                cert.id,
                cert.spiffe_id.to_string(),
                cert.serial_number,
                cert.pem,
                cert.issued_at.to_rfc3339(),
                cert.expires_at.to_rfc3339(),
                cert.status.to_string(),
            ],
        )
        .map_err(|e| Error::Database(format!("插入证书失败: {}", e)))?;
        Ok(())
    }

    /// 按 SPIFFE ID 查询证书
    pub fn get_certificates_by_spiffe(&self, spiffe_id: &SpiffeId) -> Result<Vec<Certificate>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare(
                r#"SELECT id, spiffe_id, serial_number, pem, issued_at, expires_at, status
                   FROM certificates WHERE spiffe_id = ?1 ORDER BY issued_at DESC"#,
            )
            .map_err(|e| Error::Database(format!("查询证书准备失败: {}", e)))?;
        let certs = stmt
            .query_map(params![spiffe_id.to_string()], |row| {
                Ok(Certificate {
                    id: row.get(0)?,
                    spiffe_id: SpiffeId::parse(&row.get::<_, String>(1)?).unwrap_or_else(|_| spiffe_id.clone()),
                    serial_number: row.get(2)?,
                    pem: row.get(3)?,
                    issued_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?).map(|dt| dt.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
                    expires_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?).map(|dt| dt.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
                    status: match row.get::<_, String>(6)?.as_str() {
                        "active" => CertificateStatus::Active,
                        "revoked" => CertificateStatus::Revoked,
                        _ => CertificateStatus::Expired,
                    },
                })
            })
            .map_err(|e| Error::Database(format!("查询证书失败: {}", e)))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(certs)
    }

    /// 获取活跃证书
    pub fn get_active_certificate(&self, spiffe_id: &SpiffeId) -> Result<Option<Certificate>> {
        let certs = self.get_certificates_by_spiffe(spiffe_id)?;
        Ok(certs.into_iter().find(|c| c.status == CertificateStatus::Active))
    }

    /// 列出所有证书
    pub fn list_certificates(&self, limit: u32, offset: u32) -> Result<Vec<Certificate>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare(
                r#"SELECT id, spiffe_id, serial_number, pem, issued_at, expires_at, status
                   FROM certificates ORDER BY issued_at DESC LIMIT ?1 OFFSET ?2"#,
            )
            .map_err(|e| Error::Database(format!("准备证书查询失败: {}", e)))?;
        let certs = stmt
            .query_map(params![limit as i64, offset as i64], |row| {
                Ok(Certificate {
                    id: row.get(0)?,
                    spiffe_id: SpiffeId::parse(&row.get::<_, String>(1)?).unwrap_or_else(|_| SpiffeId::for_device("unknown", "unknown")),
                    serial_number: row.get(2)?,
                    pem: row.get(3)?,
                    issued_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?).map(|dt| dt.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
                    expires_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?).map(|dt| dt.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
                    status: match row.get::<_, String>(6)?.as_str() {
                        "active" => CertificateStatus::Active,
                        "revoked" => CertificateStatus::Revoked,
                        _ => CertificateStatus::Expired,
                    },
                })
            })
            .map_err(|e| Error::Database(format!("查询证书列表失败: {}", e)))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(certs)
    }

    /// 更新证书状态
    pub fn update_certificate_status(
        &self,
        cert_id: &str,
        status: &CertificateStatus,
    ) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "UPDATE certificates SET status = ?1 WHERE id = ?2",
            params![status.to_string(), cert_id],
        )
        .map_err(|e| Error::Database(format!("更新证书状态失败: {}", e)))?;
        Ok(())
    }

    /// 获取即将过期的证书（在指定天数内）
    pub fn get_expiring_certificates(&self, days: u32) -> Result<Vec<Certificate>> {
        let deadline = Utc::now() + chrono::Duration::days(days as i64);
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare(
                r#"SELECT id, spiffe_id, serial_number, pem, issued_at, expires_at, status
                   FROM certificates WHERE status = 'active' AND expires_at < ?1 ORDER BY expires_at"#,
            )
            .map_err(|e| Error::Database(format!("准备过期证书查询失败: {}", e)))?;
        let certs = stmt
            .query_map(params![deadline.to_rfc3339()], |row| {
                Ok(Certificate {
                    id: row.get(0)?,
                    spiffe_id: SpiffeId::parse(&row.get::<_, String>(1)?).unwrap_or_else(|_| SpiffeId::for_device("unknown", "unknown")),
                    serial_number: row.get(2)?,
                    pem: row.get(3)?,
                    issued_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?).map(|dt| dt.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
                    expires_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?).map(|dt| dt.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
                    status: match row.get::<_, String>(6)?.as_str() {
                        "active" => CertificateStatus::Active,
                        "revoked" => CertificateStatus::Revoked,
                        _ => CertificateStatus::Expired,
                    },
                })
            })
            .map_err(|e| Error::Database(format!("查询过期证书失败: {}", e)))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(certs)
    }

    /// 吊销证书
    pub fn revoke_certificate(&self, cert_id: &str) -> Result<()> {
        self.update_certificate_status(cert_id, &CertificateStatus::Revoked)
    }

    /// 按序列号吊销所有证书
    pub fn revoke_certificates_by_spiffe(&self, spiffe_id: &SpiffeId) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "UPDATE certificates SET status = 'revoked' WHERE spiffe_id = ?1 AND status = 'active'",
            params![spiffe_id.to_string()],
        )
        .map_err(|e| Error::Database(format!("吊销证书失败: {}", e)))?;
        Ok(())
    }

    /// 标记过期证书
    pub fn mark_expired_certificates(&self) -> Result<u64> {
        let conn = self.conn()?;
        let count = conn.execute(
            "UPDATE certificates SET status = 'expired' WHERE status = 'active' AND expires_at < ?1",
            params![Utc::now().to_rfc3339()],
        )
        .map_err(|e| Error::Database(format!("标记过期证书失败: {}", e)))?;
        Ok(count as u64)
    }

    /// 列出所有已吊销证书（用于 CRL 生成）
    pub fn list_revoked_certificates(&self) -> Result<Vec<RevokedCertInfo>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare(
                r#"SELECT serial_number, spiffe_id, expires_at
                   FROM certificates WHERE status = 'revoked' ORDER BY expires_at"#,
            )
            .map_err(|e| Error::Database(format!("准备吊销证书查询失败: {}", e)))?;
        let certs = stmt
            .query_map([], |row| {
                Ok(RevokedCertInfo {
                    serial_number: row.get(0)?,
                    spiffe_id: row.get(1)?,
                    expires_at: row.get(2)?,
                    revoked_at: row.get::<_, String>(2).unwrap_or_default(),
                })
            })
            .map_err(|e| Error::Database(format!("查询吊销证书失败: {}", e)))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(certs)
    }
}

/// 已吊销证书信息（用于 CRL）
#[derive(Debug, Clone, serde::Serialize)]
pub struct RevokedCertInfo {
    pub serial_number: String,
    pub spiffe_id: String,
    pub expires_at: String,
    pub revoked_at: String,
}
