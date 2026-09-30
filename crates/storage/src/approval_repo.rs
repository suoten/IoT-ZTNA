//! 策略审批仓储
//!
//! 多级审批流程: 操作员提交 → 管理员审批 → 发布
//! 审批状态: pending → approved/rejected → published

use crate::Database;
use chrono::{DateTime, Utc};
use iot_ztna_common::{Error, Result};
use rusqlite::params;
use serde::{Deserialize, Serialize};

/// 审批状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApprovalStatus {
    /// 待审批
    Pending,
    /// 已批准
    Approved,
    /// 已拒绝
    Rejected,
    /// 已发布
    Published,
}

impl std::fmt::Display for ApprovalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApprovalStatus::Pending => write!(f, "pending"),
            ApprovalStatus::Approved => write!(f, "approved"),
            ApprovalStatus::Rejected => write!(f, "rejected"),
            ApprovalStatus::Published => write!(f, "published"),
        }
    }
}

impl std::str::FromStr for ApprovalStatus {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "pending" => Ok(ApprovalStatus::Pending),
            "approved" => Ok(ApprovalStatus::Approved),
            "rejected" => Ok(ApprovalStatus::Rejected),
            "published" => Ok(ApprovalStatus::Published),
            _ => Err(format!("未知审批状态: {}", s)),
        }
    }
}

/// 策略审批记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyApproval {
    pub id: String,
    pub policy_id: String,
    pub policy_name: String,
    pub rego_content: String,
    pub status: ApprovalStatus,
    pub submitted_by: String,
    pub submitted_at: DateTime<Utc>,
    pub reviewed_by: Option<String>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub review_comment: Option<String>,
}

impl Database {
    /// 创建策略审批请求
    pub fn create_policy_approval(
        &self,
        policy_id: &str,
        policy_name: &str,
        rego_content: &str,
        submitted_by: &str,
    ) -> Result<String> {
        let conn = self.conn()?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        conn.execute(
            r#"INSERT INTO policy_approvals
               (id, policy_id, policy_name, rego_content, status, submitted_by, submitted_at)
               VALUES (?1, ?2, ?3, ?4, 'pending', ?5, ?6)"#,
            params![id, policy_id, policy_name, rego_content, submitted_by, now],
        )
        .map_err(|e| Error::Database(format!("创建审批请求失败: {}", e)))?;

        Ok(id)
    }

    /// 列出待审批请求
    pub fn list_pending_approvals(&self) -> Result<Vec<PolicyApproval>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare(
                r#"SELECT id, policy_id, policy_name, rego_content, status,
                          submitted_by, submitted_at, reviewed_by, reviewed_at, review_comment
                   FROM policy_approvals
                   WHERE status = 'pending'
                   ORDER BY submitted_at DESC"#,
            )
            .map_err(|e| Error::Database(format!("准备审批查询失败: {}", e)))?;

        let approvals = stmt
            .query_map([], |row| {
                Ok(PolicyApproval {
                    id: row.get(0)?,
                    policy_id: row.get(1)?,
                    policy_name: row.get(2)?,
                    rego_content: row.get(3)?,
                    status: row.get::<_, String>(4)?.parse().unwrap_or(ApprovalStatus::Pending),
                    submitted_by: row.get(5)?,
                    submitted_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    reviewed_by: row.get::<_, Option<String>>(7)?,
                    reviewed_at: row
                        .get::<_, Option<String>>(8)?
                        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&Utc)),
                    review_comment: row.get::<_, Option<String>>(9)?,
                })
            })
            .map_err(|e| Error::Database(format!("查询审批列表失败: {}", e)))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(approvals)
    }

    /// 列出所有审批记录
    pub fn list_all_approvals(&self) -> Result<Vec<PolicyApproval>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare(
                r#"SELECT id, policy_id, policy_name, rego_content, status,
                          submitted_by, submitted_at, reviewed_by, reviewed_at, review_comment
                   FROM policy_approvals
                   ORDER BY submitted_at DESC"#,
            )
            .map_err(|e| Error::Database(format!("准备审批查询失败: {}", e)))?;

        let approvals = stmt
            .query_map([], |row| {
                Ok(PolicyApproval {
                    id: row.get(0)?,
                    policy_id: row.get(1)?,
                    policy_name: row.get(2)?,
                    rego_content: row.get(3)?,
                    status: row.get::<_, String>(4)?.parse().unwrap_or(ApprovalStatus::Pending),
                    submitted_by: row.get(5)?,
                    submitted_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    reviewed_by: row.get::<_, Option<String>>(7)?,
                    reviewed_at: row
                        .get::<_, Option<String>>(8)?
                        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&Utc)),
                    review_comment: row.get::<_, Option<String>>(9)?,
                })
            })
            .map_err(|e| Error::Database(format!("查询审批列表失败: {}", e)))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(approvals)
    }

    /// 审批策略（批准/拒绝）
    pub fn review_policy_approval(
        &self,
        approval_id: &str,
        approved: bool,
        reviewed_by: &str,
        comment: &str,
    ) -> Result<()> {
        let conn = self.conn()?;
        let now = Utc::now().to_rfc3339();
        let status = if approved { "approved" } else { "rejected" };

        conn.execute(
            r#"UPDATE policy_approvals
               SET status = ?1, reviewed_by = ?2, reviewed_at = ?3, review_comment = ?4
               WHERE id = ?5 AND status = 'pending'"#,
            params![status, reviewed_by, now, comment, approval_id],
        )
        .map_err(|e| Error::Database(format!("审批更新失败: {}", e)))?;

        if approved {
            // 审批通过后自动发布策略
            conn.execute(
                r#"UPDATE policy_approvals SET status = 'published' WHERE id = ?1"#,
                params![approval_id],
            )
            .map_err(|e| Error::Database(format!("发布审批失败: {}", e)))?;
        }

        Ok(())
    }

    /// 获取审批详情
    pub fn get_policy_approval(&self, approval_id: &str) -> Result<Option<PolicyApproval>> {
        let conn = self.conn()?;
        let approval = conn
            .query_row(
                r#"SELECT id, policy_id, policy_name, rego_content, status,
                          submitted_by, submitted_at, reviewed_by, reviewed_at, review_comment
                   FROM policy_approvals WHERE id = ?1"#,
                params![approval_id],
                |row| {
                    Ok(PolicyApproval {
                        id: row.get(0)?,
                        policy_id: row.get(1)?,
                        policy_name: row.get(2)?,
                        rego_content: row.get(3)?,
                        status: row.get::<_, String>(4)?.parse().unwrap_or(ApprovalStatus::Pending),
                        submitted_by: row.get(5)?,
                        submitted_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                        reviewed_by: row.get::<_, Option<String>>(7)?,
                        reviewed_at: row
                            .get::<_, Option<String>>(8)?
                            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                            .map(|dt| dt.with_timezone(&Utc)),
                        review_comment: row.get::<_, Option<String>>(9)?,
                    })
                },
            )
            .ok();
        Ok(approval)
    }
}
