//! IoT-ZTNA 存储层
//!
//! 基于 SQLite (rusqlite) 的本地存储，WAL 模式，支持设备、证书、策略、告警、审计日志的 CRUD。

pub mod migrations;
pub mod device_repo;
pub mod cert_repo;
pub mod policy_repo;
pub mod alert_repo;
pub mod audit_repo;
pub mod approval_repo;
pub mod model_repo;

pub use policy_repo::PolicyVersion;
pub use cert_repo::RevokedCertInfo;
pub use approval_repo::{PolicyApproval, ApprovalStatus};
pub use model_repo::ModelVersionRecord;

use iot_ztna_common::Error;

use rusqlite::{Connection, OpenFlags};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// 线程安全的 SQLite 连接池
#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    /// 打开数据库并初始化
    pub fn open(path: &Path) -> Result<Self, Error> {
        // 确保父目录存在
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                Error::Config(format!("创建数据库目录失败: {}", e))
            })?;
        }

        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|e| Error::Database(format!("打开数据库失败: {}", e)))?;

        // 启用 WAL 模式
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| Error::Database(format!("设置 WAL 模式失败: {}", e)))?;
        // 外键约束
        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(|e| Error::Database(format!("启用外键约束失败: {}", e)))?;
        // 忙等超时
        conn.pragma_update(None, "busy_timeout", "5000")
            .map_err(|e| Error::Database(format!("设置忙等超时失败: {}", e)))?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        // 执行迁移
        db.migrate()?;

        Ok(db)
    }

    /// 打开内存数据库（用于测试）
    pub fn open_memory() -> Result<Self, Error> {
        let conn = Connection::open_in_memory()
            .map_err(|e| Error::Database(format!("打开内存数据库失败: {}", e)))?;
        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(|e| Error::Database(format!("启用外键约束失败: {}", e)))?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.migrate()?;
        Ok(db)
    }

    /// 获取连接（需在外部持锁）
    pub fn conn(&self) -> Result<std::sync::MutexGuard<'_, Connection>, Error> {
        self.conn
            .lock()
            .map_err(|e| Error::Database(format!("获取数据库连接锁失败: {}", e)))
    }

    /// 执行迁移
    pub fn migrate(&self) -> Result<(), Error> {
        let conn = self.conn()?;
        migrations::run_migrations(&conn)?;
        Ok(())
    }

    /// 备份数据库
    pub fn backup(&self, dest: &Path) -> Result<(), Error> {
        let conn = self.conn()?;
        conn.execute(
            &format!("VACUUM INTO '{}'", dest.display()),
            [],
        )
        .map_err(|e| Error::Database(format!("备份失败: {}", e)))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_memory_db() {
        let db = Database::open_memory().unwrap();
        // 确保表已创建
        let conn = db.conn().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM devices", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }
}
