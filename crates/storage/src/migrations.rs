//! 数据库迁移

use iot_ztna_common::Error;
use rusqlite::Connection;

const MIGRATIONS: &[&str] = &[
    // Migration 001: 初始 schema
    r#"
    -- 设备表
    CREATE TABLE IF NOT EXISTS devices (
        id TEXT PRIMARY KEY,
        spiffe_id TEXT NOT NULL UNIQUE,
        serial_number TEXT NOT NULL UNIQUE,
        model TEXT NOT NULL,
        trust_root TEXT NOT NULL,
        trust_level TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'pending',
        capabilities TEXT NOT NULL DEFAULT '[]',
        hardware_fingerprint TEXT,
        registered_at TEXT NOT NULL,
        last_seen TEXT,
        maintenance_mode INTEGER NOT NULL DEFAULT 0
    );

    -- 证书表
    CREATE TABLE IF NOT EXISTS certificates (
        id TEXT PRIMARY KEY,
        spiffe_id TEXT NOT NULL,
        serial_number TEXT NOT NULL UNIQUE,
        pem TEXT NOT NULL,
        private_key TEXT,
        issued_at TEXT NOT NULL,
        expires_at TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'active',
        FOREIGN KEY (spiffe_id) REFERENCES devices(spiffe_id)
    );

    -- 策略表
    CREATE TABLE IF NOT EXISTS policies (
        id TEXT PRIMARY KEY,
        version TEXT NOT NULL,
        name TEXT NOT NULL,
        rego_content TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'draft',
        created_by TEXT NOT NULL,
        created_at TEXT NOT NULL,
        published_at TEXT
    );

    -- 告警表
    CREATE TABLE IF NOT EXISTS alerts (
        id TEXT PRIMARY KEY,
        alert_type TEXT NOT NULL,
        severity TEXT NOT NULL,
        spiffe_id TEXT,
        message TEXT NOT NULL,
        details TEXT NOT NULL DEFAULT '{}',
        created_at TEXT NOT NULL,
        acknowledged INTEGER NOT NULL DEFAULT 0
    );

    -- 审计日志表（含哈希链防篡改）
    CREATE TABLE IF NOT EXISTS audit_logs (
        id TEXT PRIMARY KEY,
        timestamp TEXT NOT NULL,
        actor TEXT NOT NULL,
        action TEXT NOT NULL,
        target TEXT NOT NULL,
        result TEXT NOT NULL,
        details TEXT NOT NULL DEFAULT '{}',
        ip_address TEXT,
        prev_hash TEXT NOT NULL DEFAULT 'GENESIS',
        hash TEXT NOT NULL DEFAULT ''
    );

    -- 策略版本历史表（版本管理与回滚）
    CREATE TABLE IF NOT EXISTS policy_versions (
        id TEXT PRIMARY KEY,
        policy_id TEXT NOT NULL,
        version TEXT NOT NULL,
        rego_content TEXT NOT NULL,
        created_by TEXT NOT NULL,
        created_at TEXT NOT NULL,
        FOREIGN KEY (policy_id) REFERENCES policies(id)
    );

    -- 注册令牌表
    CREATE TABLE IF NOT EXISTS registration_tokens (
        id TEXT PRIMARY KEY,
        token TEXT NOT NULL UNIQUE,
        serial_number TEXT NOT NULL,
        hardware_fingerprint TEXT NOT NULL,
        expires_at TEXT NOT NULL,
        used INTEGER NOT NULL DEFAULT 0,
        created_at TEXT NOT NULL
    );

    -- 索引
    CREATE INDEX IF NOT EXISTS idx_devices_status ON devices(status);
    CREATE INDEX IF NOT EXISTS idx_devices_serial ON devices(serial_number);
    CREATE INDEX IF NOT EXISTS idx_certs_spiffe ON certificates(spiffe_id);
    CREATE INDEX IF NOT EXISTS idx_certs_status ON certificates(status);
    CREATE INDEX IF NOT EXISTS idx_certs_expires ON certificates(expires_at);
    CREATE INDEX IF NOT EXISTS idx_policies_status ON policies(status);
    CREATE INDEX IF NOT EXISTS idx_alerts_severity ON alerts(severity);
    CREATE INDEX IF NOT EXISTS idx_alerts_created ON alerts(created_at);
    CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_logs(timestamp);
    CREATE INDEX IF NOT EXISTS idx_audit_actor ON audit_logs(actor);
    "#,
    // Migration 002: 用户管理表
    r#"
    -- 用户表
    CREATE TABLE IF NOT EXISTS users (
        id TEXT PRIMARY KEY,
        username TEXT NOT NULL UNIQUE,
        password_hash TEXT NOT NULL,
        role TEXT NOT NULL DEFAULT 'operator',
        status TEXT NOT NULL DEFAULT 'active',
        created_at TEXT NOT NULL,
        updated_at TEXT,
        last_login TEXT
    );

    -- 用户表索引
    CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
    "#,
    // Migration 003: 策略审批 + 模型版本管理
    r#"
    -- 策略审批流程表
    CREATE TABLE IF NOT EXISTS policy_approvals (
        id TEXT PRIMARY KEY,
        policy_id TEXT NOT NULL,
        policy_name TEXT NOT NULL,
        rego_content TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'pending',
        submitted_by TEXT NOT NULL,
        submitted_at TEXT NOT NULL,
        reviewed_by TEXT,
        reviewed_at TEXT,
        review_comment TEXT,
        FOREIGN KEY (policy_id) REFERENCES policies(id)
    );

    CREATE INDEX IF NOT EXISTS idx_approvals_status ON policy_approvals(status);
    CREATE INDEX IF NOT EXISTS idx_approvals_policy ON policy_approvals(policy_id);

    -- 模型版本管理表
    CREATE TABLE IF NOT EXISTS model_versions (
        id TEXT PRIMARY KEY,
        version TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'ready',
        epochs INTEGER NOT NULL DEFAULT 0,
        training_samples INTEGER NOT NULL DEFAULT 0,
        validation_loss REAL NOT NULL DEFAULT 0.0,
        created_by TEXT NOT NULL,
        description TEXT NOT NULL DEFAULT '',
        calibration_json TEXT NOT NULL DEFAULT '{}',
        created_at TEXT NOT NULL,
        activated_at TEXT
    );

    CREATE INDEX IF NOT EXISTS idx_model_versions_status ON model_versions(status);

    -- OPA Bundle 同步记录
    CREATE TABLE IF NOT EXISTS opa_bundles (
        id TEXT PRIMARY KEY,
        revision TEXT NOT NULL,
        content_hash TEXT NOT NULL,
        size_bytes INTEGER NOT NULL,
        synced_at TEXT NOT NULL,
        active INTEGER NOT NULL DEFAULT 0
    );

    CREATE INDEX IF NOT EXISTS idx_opa_bundles_active ON opa_bundles(active);
    "#,
];

/// 运行所有迁移
pub fn run_migrations(conn: &Connection) -> Result<(), Error> {
    // 创建 schema_migrations 表
    conn.execute(
        r#"CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL
        )"#,
        [],
    )
    .map_err(|e| Error::Database(format!("创建 migration 表失败: {}", e)))?;

    for (i, sql) in MIGRATIONS.iter().enumerate() {
        let version = (i + 1) as i64;
        let applied: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version = ?1",
                rusqlite::params![version],
                |row| row.get(0),
            )
            .unwrap_or(0);
        if applied > 0 {
            continue;
        }
        tracing::info!("执行数据库迁移版本 {}", version);
        conn.execute_batch(sql).map_err(|e| {
            Error::Database(format!("迁移 {} 失败: {}", version, e))
        })?;
        conn.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![version, chrono::Utc::now().to_rfc3339()],
        )
        .map_err(|e| Error::Database(format!("记录迁移状态失败: {}", e)))?;
    }
    Ok(())
}
