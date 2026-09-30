//! 告警仓储

use crate::Database;
use chrono::{DateTime, Utc};
use iot_ztna_common::{Alert, AlertSeverity, AlertType, Error, Result, SpiffeId};
use rusqlite::params;

impl Database {
    /// 插入告警
    pub fn insert_alert(&self, alert: &Alert) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            r#"INSERT INTO alerts
               (id, alert_type, severity, spiffe_id, message, details, created_at, acknowledged)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"#,
            params![
                alert.id,
                serde_json::to_string(&alert.alert_type).unwrap_or_default(),
                serde_json::to_string(&alert.severity).unwrap_or_default(),
                alert.spiffe_id.as_ref().map(|s| s.to_string()),
                alert.message,
                alert.details.to_string(),
                alert.created_at.to_rfc3339(),
                alert.acknowledged as i32,
            ],
        )
        .map_err(|e| Error::Database(format!("插入告警失败: {}", e)))?;
        Ok(())
    }

    /// 列出告警
    pub fn list_alerts(&self, limit: u32, offset: u32) -> Result<Vec<Alert>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare(
                r#"SELECT id, alert_type, severity, spiffe_id, message, details, created_at, acknowledged
                   FROM alerts ORDER BY created_at DESC LIMIT ?1 OFFSET ?2"#,
            )
            .map_err(|e| Error::Database(format!("准备告警查询失败: {}", e)))?;
        let alerts = stmt
            .query_map(params![limit as i64, offset as i64], |row| {
                let alert_type_str: String = row.get(1)?;
                let severity_str: String = row.get(2)?;
                Ok(Alert {
                    id: row.get(0)?,
                    alert_type: serde_json::from_str(&alert_type_str).unwrap_or(AlertType::SystemError),
                    severity: serde_json::from_str(&severity_str).unwrap_or(AlertSeverity::Info),
                    spiffe_id: row.get::<_, Option<String>>(3)?.and_then(|s| SpiffeId::parse(&s).ok()),
                    message: row.get(4)?,
                    details: serde_json::from_str(&row.get::<_, String>(5).unwrap_or_default()).unwrap_or_default(),
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?).map(|dt| dt.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
                    acknowledged: row.get::<_, i32>(7)? != 0,
                })
            })
            .map_err(|e| Error::Database(format!("查询告警失败: {}", e)))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(alerts)
    }

    /// 确认告警
    pub fn acknowledge_alert(&self, alert_id: &str) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "UPDATE alerts SET acknowledged = 1 WHERE id = ?1",
            params![alert_id],
        )
        .map_err(|e| Error::Database(format!("确认告警失败: {}", e)))?;
        Ok(())
    }

    /// 统计未确认告警
    pub fn count_unacknowledged_alerts(&self) -> Result<u64> {
        let conn = self.conn()?;
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM alerts WHERE acknowledged = 0",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        Ok(count as u64)
    }

    /// 统计拦截次数
    pub fn count_denials(&self) -> Result<u64> {
        let conn = self.conn()?;
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM alerts WHERE alert_type LIKE '%policy_denied%' OR alert_type LIKE '%unauthorized%'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        Ok(count as u64)
    }
}
