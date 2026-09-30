//! 策略仓储

use crate::Database;
use chrono::{DateTime, Utc};
use iot_ztna_common::{Error, Policy, PolicyStatus, Result};
use rusqlite::params;

impl Database {
    /// 插入策略
    pub fn insert_policy(&self, policy: &Policy) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            r#"INSERT INTO policies
               (id, version, name, rego_content, status, created_by, created_at, published_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"#,
            params![
                policy.id,
                policy.version,
                policy.name,
                policy.rego_content,
                policy.status.to_string(),
                policy.created_by,
                policy.created_at.to_rfc3339(),
                policy.published_at.map(|t| t.to_rfc3339()),
            ],
        )
        .map_err(|e| Error::Database(format!("插入策略失败: {}", e)))?;
        Ok(())
    }

    /// 获取已发布策略
    pub fn get_published_policies(&self) -> Result<Vec<Policy>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare(
                r#"SELECT id, version, name, rego_content, status, created_by, created_at, published_at
                   FROM policies WHERE status = 'published' ORDER BY published_at DESC"#,
            )
            .map_err(|e| Error::Database(format!("准备策略查询失败: {}", e)))?;
        let policies = stmt
            .query_map([], |row| {
                Ok(Policy {
                    id: row.get(0)?,
                    version: row.get(1)?,
                    name: row.get(2)?,
                    rego_content: row.get(3)?,
                    status: match row.get::<_, String>(4)?.as_str() {
                        "published" => PolicyStatus::Published,
                        "archived" => PolicyStatus::Archived,
                        _ => PolicyStatus::Draft,
                    },
                    created_by: row.get(5)?,
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?).map(|dt| dt.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
                    published_at: row.get::<_, Option<String>>(7)?.and_then(|s| DateTime::parse_from_rfc3339(&s).ok()).map(|dt| dt.with_timezone(&Utc)),
                })
            })
            .map_err(|e| Error::Database(format!("查询策略失败: {}", e)))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(policies)
    }

    /// 列出所有策略
    pub fn list_policies(&self, limit: u32, offset: u32) -> Result<Vec<Policy>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare(
                r#"SELECT id, version, name, rego_content, status, created_by, created_at, published_at
                   FROM policies ORDER BY created_at DESC LIMIT ?1 OFFSET ?2"#,
            )
            .map_err(|e| Error::Database(format!("准备策略查询失败: {}", e)))?;
        let policies = stmt
            .query_map(params![limit as i64, offset as i64], |row| {
                Ok(Policy {
                    id: row.get(0)?,
                    version: row.get(1)?,
                    name: row.get(2)?,
                    rego_content: row.get(3)?,
                    status: match row.get::<_, String>(4)?.as_str() {
                        "published" => PolicyStatus::Published,
                        "archived" => PolicyStatus::Archived,
                        _ => PolicyStatus::Draft,
                    },
                    created_by: row.get(5)?,
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?).map(|dt| dt.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
                    published_at: row.get::<_, Option<String>>(7)?.and_then(|s| DateTime::parse_from_rfc3339(&s).ok()).map(|dt| dt.with_timezone(&Utc)),
                })
            })
            .map_err(|e| Error::Database(format!("查询策略列表失败: {}", e)))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(policies)
    }

    /// 获取单个策略
    pub fn get_policy(&self, id: &str) -> Result<Option<Policy>> {
        let conn = self.conn()?;
        let policy = conn
            .query_row(
                r#"SELECT id, version, name, rego_content, status, created_by, created_at, published_at
                   FROM policies WHERE id = ?1"#,
                params![id],
                |row| {
                    Ok(Policy {
                        id: row.get(0)?,
                        version: row.get(1)?,
                        name: row.get(2)?,
                        rego_content: row.get(3)?,
                        status: match row.get::<_, String>(4)?.as_str() {
                            "published" => PolicyStatus::Published,
                            "archived" => PolicyStatus::Archived,
                            _ => PolicyStatus::Draft,
                        },
                        created_by: row.get(5)?,
                        created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?).map(|dt| dt.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
                        published_at: row.get::<_, Option<String>>(7)?.and_then(|s| DateTime::parse_from_rfc3339(&s).ok()).map(|dt| dt.with_timezone(&Utc)),
                    })
                },
            )
            .ok();
        Ok(policy)
    }

    /// 更新策略状态
    pub fn update_policy_status(&self, id: &str, status: &PolicyStatus) -> Result<()> {
        let conn = self.conn()?;
        let published_at = if *status == PolicyStatus::Published {
            Some(Utc::now().to_rfc3339())
        } else {
            None
        };
        conn.execute(
            "UPDATE policies SET status = ?1, published_at = COALESCE(?2, published_at) WHERE id = ?3",
            params![status.to_string(), published_at, id],
        )
        .map_err(|e| Error::Database(format!("更新策略状态失败: {}", e)))?;
        Ok(())
    }

    /// 删除策略
    pub fn delete_policy(&self, id: &str) -> Result<()> {
        let conn = self.conn()?;
        conn.execute("DELETE FROM policies WHERE id = ?1", params![id])
            .map_err(|e| Error::Database(format!("删除策略失败: {}", e)))?;
        Ok(())
    }

    /// 更新策略内容（用于 PUT 更新和版本回滚）
    pub fn update_policy_content(
        &self,
        id: &str,
        rego_content: &str,
        updated_by: &str,
    ) -> Result<()> {
        let conn = self.conn()?;
        // 递增版本号
        conn.execute(
            r#"UPDATE policies 
               SET rego_content = ?1, 
                   version = CAST(CAST(version AS REAL) + 0.1 AS TEXT),
                   created_by = ?2
               WHERE id = ?3"#,
            params![rego_content, updated_by, id],
        )
        .map_err(|e| Error::Database(format!("更新策略内容失败: {}", e)))?;
        Ok(())
    }

    /// 保存策略版本历史
    pub fn save_policy_version(
        &self,
        policy_id: &str,
        version: &str,
        rego_content: &str,
        created_by: &str,
    ) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            r#"INSERT INTO policy_versions (id, policy_id, version, rego_content, created_by, created_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
            params![
                uuid::Uuid::new_v4().to_string(),
                policy_id,
                version,
                rego_content,
                created_by,
                Utc::now().to_rfc3339(),
            ],
        )
        .map_err(|e| Error::Database(format!("保存策略版本失败: {}", e)))?;
        Ok(())
    }

    /// 列出策略版本历史
    pub fn list_policy_versions(&self, policy_id: &str) -> Result<Vec<PolicyVersion>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare(
                r#"SELECT id, policy_id, version, rego_content, created_by, created_at
                   FROM policy_versions WHERE policy_id = ?1 ORDER BY created_at DESC"#,
            )
            .map_err(|e| Error::Database(format!("准备策略版本查询失败: {}", e)))?;
        let versions = stmt
            .query_map(params![policy_id], |row| {
                Ok(PolicyVersion {
                    id: row.get(0)?,
                    policy_id: row.get(1)?,
                    version: row.get(2)?,
                    rego_content: row.get(3)?,
                    created_by: row.get(4)?,
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })
            .map_err(|e| Error::Database(format!("查询策略版本失败: {}", e)))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(versions)
    }

    /// 回滚策略到指定版本
    pub fn rollback_policy(&self, policy_id: &str, version_id: &str) -> Result<()> {
        let conn = self.conn()?;
        let version_content: String = conn
            .query_row(
                "SELECT rego_content FROM policy_versions WHERE id = ?1 AND policy_id = ?2",
                params![version_id, policy_id],
                |row| row.get(0),
            )
            .map_err(|e| Error::Database(format!("查询版本内容失败: {}", e)))?;

        // 更新策略内容为历史版本
        conn.execute(
            "UPDATE policies SET rego_content = ?1 WHERE id = ?2",
            params![version_content, policy_id],
        )
        .map_err(|e| Error::Database(format!("回滚策略失败: {}", e)))?;
        Ok(())
    }
}

/// 策略版本历史记录
#[derive(Debug, Clone, serde::Serialize)]
pub struct PolicyVersion {
    pub id: String,
    pub policy_id: String,
    pub version: String,
    pub rego_content: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}
