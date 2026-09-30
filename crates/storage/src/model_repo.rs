//! 模型版本仓储
//!
//! 存储 Autoencoder 模型版本信息，支持版本列表、激活、回滚。

use crate::Database;
use chrono::{DateTime, Utc};
use iot_ztna_common::{Error, Result};
use rusqlite::params;
use serde::{Deserialize, Serialize};

/// 模型版本记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelVersionRecord {
    pub id: String,
    pub version: String,
    pub status: String,
    pub epochs: i64,
    pub training_samples: i64,
    pub validation_loss: f64,
    pub created_by: String,
    pub description: String,
    pub calibration_json: String,
    pub created_at: DateTime<Utc>,
    pub activated_at: Option<DateTime<Utc>>,
}

impl Database {
    /// 插入模型版本
    pub fn insert_model_version(
        &self,
        id: &str,
        version: &str,
        epochs: usize,
        training_samples: usize,
        validation_loss: f64,
        created_by: &str,
        description: &str,
        calibration_json: &str,
    ) -> Result<()> {
        let conn = self.conn()?;
        let now = Utc::now().to_rfc3339();

        conn.execute(
            r#"INSERT INTO model_versions
               (id, version, status, epochs, training_samples, validation_loss,
                created_by, description, calibration_json, created_at)
               VALUES (?1, ?2, 'ready', ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#,
            params![
                id,
                version,
                epochs as i64,
                training_samples as i64,
                validation_loss,
                created_by,
                description,
                calibration_json,
                now,
            ],
        )
        .map_err(|e| Error::Database(format!("插入模型版本失败: {}", e)))?;

        Ok(())
    }

    /// 列出所有模型版本
    pub fn list_model_versions(&self) -> Result<Vec<ModelVersionRecord>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare(
                r#"SELECT id, version, status, epochs, training_samples,
                          validation_loss, created_by, description, calibration_json,
                          created_at, activated_at
                   FROM model_versions
                   ORDER BY created_at DESC"#,
            )
            .map_err(|e| Error::Database(format!("准备模型版本查询失败: {}", e)))?;

        let versions = stmt
            .query_map([], |row| {
                Ok(ModelVersionRecord {
                    id: row.get(0)?,
                    version: row.get(1)?,
                    status: row.get(2)?,
                    epochs: row.get(3)?,
                    training_samples: row.get(4)?,
                    validation_loss: row.get(5)?,
                    created_by: row.get(6)?,
                    description: row.get(7)?,
                    calibration_json: row.get(8)?,
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    activated_at: row
                        .get::<_, Option<String>>(10)?
                        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&Utc)),
                })
            })
            .map_err(|e| Error::Database(format!("查询模型版本失败: {}", e)))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(versions)
    }

    /// 激活模型版本
    pub fn activate_model_version(&self, id: &str) -> Result<()> {
        let conn = self.conn()?;
        let now = Utc::now().to_rfc3339();

        // 将其他活跃版本设为已归档
        conn.execute(
            "UPDATE model_versions SET status = 'archived' WHERE status = 'active'",
            [],
        )
        .map_err(|e| Error::Database(format!("归档旧模型失败: {}", e)))?;

        // 激活新版本
        conn.execute(
            "UPDATE model_versions SET status = 'active', activated_at = ?1 WHERE id = ?2",
            params![now, id],
        )
        .map_err(|e| Error::Database(format!("激活模型失败: {}", e)))?;

        Ok(())
    }

    /// 删除模型版本
    pub fn delete_model_version(&self, id: &str) -> Result<()> {
        let conn = self.conn()?;
        conn.execute("DELETE FROM model_versions WHERE id = ?1", params![id])
            .map_err(|e| Error::Database(format!("删除模型版本失败: {}", e)))?;
        Ok(())
    }

    /// 插入 OPA Bundle 同步记录
    pub fn insert_opa_bundle(
        &self,
        id: &str,
        revision: &str,
        content_hash: &str,
        size_bytes: usize,
    ) -> Result<()> {
        let conn = self.conn()?;
        let now = Utc::now().to_rfc3339();

        // 将之前的活跃记录设为非活跃
        conn.execute("UPDATE opa_bundles SET active = 0 WHERE active = 1", [])
            .map_err(|e| Error::Database(format!("更新 OPA Bundle 状态失败: {}", e)))?;

        conn.execute(
            r#"INSERT INTO opa_bundles (id, revision, content_hash, size_bytes, synced_at, active)
               VALUES (?1, ?2, ?3, ?4, ?5, 1)"#,
            params![id, revision, content_hash, size_bytes as i64, now],
        )
        .map_err(|e| Error::Database(format!("插入 OPA Bundle 记录失败: {}", e)))?;

        Ok(())
    }

    /// 获取活跃 OPA Bundle
    pub fn get_active_opa_bundle(&self) -> Result<Option<(String, String, String, usize, DateTime<Utc>)>> {
        let conn = self.conn()?;
        let bundle = conn
            .query_row(
                "SELECT id, revision, content_hash, size_bytes, synced_at FROM opa_bundles WHERE active = 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)? as usize,
                        DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                    ))
                },
            )
            .ok();
        Ok(bundle)
    }
}
