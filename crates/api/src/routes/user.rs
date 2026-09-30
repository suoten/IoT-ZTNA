//! 用户管理路由
//!
//! 提供用户 CRUD、密码修改、角色管理 API。
//! 密码使用 PBKDF2-SHA256 进行哈希存储。

use crate::error::{AppError, AppResult};
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::Json;
use iot_ztna_common::ApiResponse;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// === 数据结构 ===

#[derive(Debug, Clone, Serialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub role: String,
    pub status: String,
    pub created_at: String,
    pub last_login: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub role: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserListResponse {
    pub users: Vec<User>,
    pub total: usize,
}

// === 密码哈希 ===

/// 使用 SHA256 加盐哈希密码（简单快速实现，适合边缘场景）
fn hash_password(password: &str) -> Result<String, String> {
    use rand::RngCore;
    // 生成随机盐
    let mut salt_bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut salt_bytes);

    // 使用简单 SHA256 加盐哈希（边缘设备性能优先）
    let mut hasher = sha2::Sha256::new();
    sha2::Digest::update(&mut hasher, &salt_bytes);
    sha2::Digest::update(&mut hasher, password.as_bytes());
    // 再哈希 100 次增加强度
    let mut hash = sha2::Digest::finalize(hasher).to_vec();
    for _ in 0..100 {
        let mut h = sha2::Sha256::new();
        sha2::Digest::update(&mut h, &hash);
        hash = sha2::Digest::finalize(h).to_vec();
    }

    Ok(format!("sha256${}${}", hex::encode(salt_bytes), hex::encode(hash)))
}

/// 验证密码
fn verify_password(password: &str, stored: &str) -> bool {
    let parts: Vec<&str> = stored.split('$').collect();
    if parts.len() != 3 || parts[0] != "sha256" {
        // 兼容旧的 pbkdf2 格式（如果存在）
        if parts.len() == 4 && parts[0] == "pbkdf2" {
            return verify_pbkdf2(password, stored);
        }
        return false;
    }
    let salt = match hex::decode(parts[1]) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let expected = match hex::decode(parts[2]) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let mut hasher = sha2::Sha256::new();
    sha2::Digest::update(&mut hasher, &salt);
    sha2::Digest::update(&mut hasher, password.as_bytes());
    let mut hash = sha2::Digest::finalize(hasher).to_vec();
    for _ in 0..100 {
        let mut h = sha2::Sha256::new();
        sha2::Digest::update(&mut h, &hash);
        hash = sha2::Digest::finalize(h).to_vec();
    }

    // 常量时间比较
    if hash.len() != expected.len() {
        return false;
    }
    let mut diff = 0u8;
    for (a, b) in hash.iter().zip(expected.iter()) {
        diff |= a ^ b;
    }
    diff == 0
}

/// 兼容旧的 PBKDF2 格式验证
fn verify_pbkdf2(password: &str, stored: &str) -> bool {
    let parts: Vec<&str> = stored.split('$').collect();
    if parts.len() != 4 {
        return false;
    }
    let iterations: usize = match parts[1].parse() {
        Ok(v) => v,
        Err(_) => return false,
    };
    let salt = match hex::decode(parts[2]) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let expected = match hex::decode(parts[3]) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let mut derived = vec![0u8; expected.len()];
    pbkdf2_hmac_sha256(password.as_bytes(), &salt, iterations, &mut derived);

    if derived.len() != expected.len() {
        return false;
    }
    let mut diff = 0u8;
    for (a, b) in derived.iter().zip(expected.iter()) {
        diff |= a ^ b;
    }
    diff == 0
}

/// PBKDF2-HMAC-SHA256 手动实现
fn pbkdf2_hmac_sha256(password: &[u8], salt: &[u8], iterations: usize, output: &mut [u8]) {
    let block_size = 32; // SHA256 输出大小
    let mut block_index = 1u32;

    for chunk in output.chunks_mut(block_size) {
        // U1 = HMAC-SHA256(password, salt || INT(block_index))
        let mut msg = salt.to_vec();
        msg.extend_from_slice(&block_index.to_be_bytes());

        let mut u = hmac_sha256(password, &msg);
        let mut result = u.clone();

        for _ in 1..iterations {
            u = hmac_sha256(password, &u);
            for (r, u_byte) in result.iter_mut().zip(u.iter()) {
                *r ^= u_byte;
            }
        }

        let copy_len = chunk.len().min(result.len());
        chunk[..copy_len].copy_from_slice(&result[..copy_len]);
        block_index += 1;
    }
}

/// HMAC-SHA256
fn hmac_sha256(key: &[u8], message: &[u8]) -> Vec<u8> {
    let block_size = 64; // SHA256 块大小

    // 如果 key 超过块大小，先哈希
    let key = if key.len() > block_size {
        let mut hasher = Sha256::new();
        hasher.update(key);
        hasher.finalize().to_vec()
    } else {
        key.to_vec()
    };

    // 填充 key 到块大小
    let mut padded_key = vec![0u8; block_size];
    padded_key[..key.len()].copy_from_slice(&key);

    // ipad 和 opad
    let ipad: Vec<u8> = padded_key.iter().map(|b| b ^ 0x36).collect();
    let opad: Vec<u8> = padded_key.iter().map(|b| b ^ 0x5c).collect();

    // inner = SHA256(ipad || message)
    let mut inner = Sha256::new();
    inner.update(&ipad);
    inner.update(message);
    let inner_hash = inner.finalize();

    // outer = SHA256(opad || inner)
    let mut outer = Sha256::new();
    outer.update(&opad);
    outer.update(&inner_hash);
    outer.finalize().to_vec()
}

// === 公共接口 ===

/// 公共密码验证函数（供外部模块调用）
pub fn verify_password_pub(password: &str, stored: &str) -> bool {
    verify_password(password, stored)
}

// === 路由处理函数 ===

/// 列出所有用户
pub async fn list_users(State(state): State<AppState>) -> AppResult<UserListResponse> {
    let conn = state.db.conn()?;
    let mut stmt = conn
        .prepare("SELECT id, username, role, status, created_at, last_login FROM users ORDER BY created_at")
        .map_err(|e| AppError::from(iot_ztna_common::Error::Database(format!("查询用户列表失败: {}", e))))?;

    let users = stmt
        .query_map([], |row| {
            Ok(User {
                id: row.get(0)?,
                username: row.get(1)?,
                role: row.get(2)?,
                status: row.get(3)?,
                created_at: row.get(4)?,
                last_login: row.get(5).ok(),
            })
        })
        .map_err(|e| AppError::from(iot_ztna_common::Error::Database(format!("查询用户列表失败: {}", e))))?
        .filter_map(|r| r.ok())
        .collect::<Vec<_>>();

    let total = users.len();
    Ok(Json(ApiResponse::ok(UserListResponse { users, total })))
}

/// 创建用户
pub async fn create_user(
    State(state): State<AppState>,
    Json(req): Json<CreateUserRequest>,
) -> AppResult<User> {
    // 验证角色
    if !["admin", "operator", "auditor"].contains(&req.role.as_str()) {
        return Err(AppError::from(iot_ztna_common::Error::Config(
            "角色必须是 admin、operator 或 auditor".to_string(),
        )));
    }

    // 验证用户名长度
    if req.username.len() < 3 || req.username.len() > 32 {
        return Err(AppError::from(iot_ztna_common::Error::Config(
            "用户名长度必须在 3-32 个字符之间".to_string(),
        )));
    }

    // 验证密码强度
    if req.password.len() < 6 {
        return Err(AppError::from(iot_ztna_common::Error::Config(
            "密码长度至少 6 个字符".to_string(),
        )));
    }

    let password_hash = hash_password(&req.password)
        .map_err(|e| AppError::from(iot_ztna_common::Error::Internal(format!("密码哈希失败: {}", e))))?;

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let conn = state.db.conn()?;
    conn.execute(
        "INSERT INTO users (id, username, password_hash, role, status, created_at) VALUES (?1, ?2, ?3, ?4, 'active', ?5)",
        rusqlite::params![id, req.username, password_hash, req.role, now],
    )
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            AppError::from(iot_ztna_common::Error::Config(
                "用户名已存在".to_string(),
            ))
        } else {
            AppError::from(iot_ztna_common::Error::Database(format!("创建用户失败: {}", e)))
        }
    })?;
    drop(conn); // 释放锁，避免 write_audit_log 死锁

    // 记录审计日志（best-effort）
    let _ = crate::routes::user::write_audit_log(
        &state.db,
        &req.username,
        "create_user",
        &format!("user:{}", req.username),
        "success",
        &serde_json::json!({"role": req.role}).to_string(),
    );

    Ok(Json(ApiResponse::ok(User {
        id,
        username: req.username,
        role: req.role,
        status: "active".to_string(),
        created_at: now,
        last_login: None,
    })))
}

/// 更新用户角色/状态
pub async fn update_user(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    Json(req): Json<UpdateUserRequest>,
) -> AppResult<()> {
    let mut updates = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(role) = &req.role {
        if !["admin", "operator", "auditor"].contains(&role.as_str()) {
            return Err(AppError::from(iot_ztna_common::Error::Config(
                "角色必须是 admin、operator 或 auditor".to_string(),
            )));
        }
        updates.push("role = ?");
        params.push(Box::new(role.clone()));
    }

    if let Some(status) = &req.status {
        if !["active", "disabled"].contains(&status.as_str()) {
            return Err(AppError::from(iot_ztna_common::Error::Config(
                "状态必须是 active 或 disabled".to_string(),
            )));
        }
        updates.push("status = ?");
        params.push(Box::new(status.clone()));
    }

    if updates.is_empty() {
        return Ok(Json(ApiResponse::ok(())));
    }

    updates.push("updated_at = ?");
    params.push(Box::new(chrono::Utc::now().to_rfc3339()));

    let set_clause = updates.join(", ");
    let sql = format!("UPDATE users SET {} WHERE id = ?", set_clause);
    params.push(Box::new(user_id.clone()));

    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let conn = state.db.conn()?;
    conn.execute(&sql, param_refs.as_slice())
        .map_err(|e| AppError::from(iot_ztna_common::Error::Database(format!("更新用户失败: {}", e))))?;

    Ok(Json(ApiResponse::ok(())))
}

/// 删除用户
pub async fn delete_user(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
) -> AppResult<()> {
    // 不允许删除最后一个 admin
    let conn = state.db.conn()?;
    let admin_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM users WHERE role = 'admin' AND status = 'active'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| AppError::from(iot_ztna_common::Error::Database(format!("查询管理员数量失败: {}", e))))?;

    let user_role: String = conn
        .query_row(
            "SELECT role FROM users WHERE id = ?1",
            rusqlite::params![user_id],
            |row| row.get(0),
        )
        .map_err(|e| AppError::from(iot_ztna_common::Error::Database(format!("查询用户失败: {}", e))))?;

    if user_role == "admin" && admin_count <= 1 {
        return Err(AppError::from(iot_ztna_common::Error::Config(
            "不能删除最后一个管理员账户".to_string(),
        )));
    }

    conn.execute("DELETE FROM users WHERE id = ?1", rusqlite::params![user_id])
        .map_err(|e| AppError::from(iot_ztna_common::Error::Database(format!("删除用户失败: {}", e))))?;

    Ok(Json(ApiResponse::ok(())))
}

/// 修改自己的密码
pub async fn change_password(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    Json(req): Json<ChangePasswordRequest>,
) -> AppResult<()> {
    if req.new_password.len() < 6 {
        return Err(AppError::from(iot_ztna_common::Error::Config(
            "新密码长度至少 6 个字符".to_string(),
        )));
    }

    let conn = state.db.conn()?;

    // 获取当前密码哈希
    let stored_hash: String = conn
        .query_row(
            "SELECT password_hash FROM users WHERE id = ?1",
            rusqlite::params![user_id],
            |row| row.get(0),
        )
        .map_err(|e| AppError::from(iot_ztna_common::Error::Database(format!("查询用户失败: {}", e))))?;

    // 验证旧密码
    if !verify_password(&req.old_password, &stored_hash) {
        return Err(AppError::from(iot_ztna_common::Error::Auth(
            "旧密码不正确".to_string(),
        )));
    }

    // 哈希新密码
    let new_hash = hash_password(&req.new_password)
        .map_err(|e| AppError::from(iot_ztna_common::Error::Internal(format!("密码哈希失败: {}", e))))?;

    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE users SET password_hash = ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![new_hash, now, user_id],
    )
    .map_err(|e| AppError::from(iot_ztna_common::Error::Database(format!("更新密码失败: {}", e))))?;

    Ok(Json(ApiResponse::ok(())))
}

/// 管理员重置用户密码
pub async fn reset_password(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    Json(req): Json<ResetPasswordRequest>,
) -> AppResult<()> {
    if req.new_password.len() < 6 {
        return Err(AppError::from(iot_ztna_common::Error::Config(
            "新密码长度至少 6 个字符".to_string(),
        )));
    }

    let new_hash = hash_password(&req.new_password)
        .map_err(|e| AppError::from(iot_ztna_common::Error::Internal(format!("密码哈希失败: {}", e))))?;

    let now = chrono::Utc::now().to_rfc3339();
    let conn = state.db.conn()?;
    conn.execute(
        "UPDATE users SET password_hash = ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![new_hash, now, user_id],
    )
    .map_err(|e| AppError::from(iot_ztna_common::Error::Database(format!("重置密码失败: {}", e))))?;

    Ok(Json(ApiResponse::ok(())))
}

/// 初始化默认管理员（如果 users 表为空）
pub fn init_default_users(db: &iot_ztna_storage::Database) -> Result<(), iot_ztna_common::Error> {
    let conn = db.conn()?;

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))
        .map_err(|e| iot_ztna_common::Error::Database(format!("查询用户数量失败: {}", e)))?;

    if count > 0 {
        tracing::info!("用户表已有 {} 个用户，跳过初始化", count);
        return Ok(());
    }

    // 从环境变量读取管理员凭据
    let admin_user = std::env::var("ZTNA_ADMIN_USER").unwrap_or_else(|_| "admin".to_string());
    let admin_pass = std::env::var("ZTNA_ADMIN_PASS").unwrap_or_else(|_| "admin123".to_string());
    let admin_role = std::env::var("ZTNA_ADMIN_ROLE").unwrap_or_else(|_| "admin".to_string());

    let password_hash = hash_password(&admin_pass)
        .map_err(|e| iot_ztna_common::Error::Internal(format!("密码哈希失败: {}", e)))?;

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO users (id, username, password_hash, role, status, created_at) VALUES (?1, ?2, ?3, ?4, 'active', ?5)",
        rusqlite::params![id, admin_user, password_hash, admin_role, now],
    )
    .map_err(|e| iot_ztna_common::Error::Database(format!("创建默认管理员失败: {}", e)))?;

    tracing::info!(username = %admin_user, "默认管理员已创建");

    Ok(())
}

/// 写审计日志（best-effort，不返回错误）
pub fn write_audit_log(
    db: &iot_ztna_storage::Database,
    actor: &str,
    action: &str,
    target: &str,
    result: &str,
    details: &str,
) {
    if let Ok(conn) = db.conn() {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();
        let now_rfc = now.to_rfc3339();
        // prev_hash 取上一条的 hash
        let prev_hash: String = conn
            .query_row(
                "SELECT hash FROM audit_logs ORDER BY timestamp DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| "GENESIS".to_string());

        // 哈希计算必须与 audit_repo.rs 的 insert_audit_log 一致:
        // SHA256(prev_hash || timestamp || actor || action || target || result || details)
        let mut hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut hasher, prev_hash.as_bytes());
        sha2::Digest::update(&mut hasher, now_rfc.as_bytes());
        sha2::Digest::update(&mut hasher, actor.as_bytes());
        sha2::Digest::update(&mut hasher, action.as_bytes());
        sha2::Digest::update(&mut hasher, target.as_bytes());
        sha2::Digest::update(&mut hasher, result.as_bytes());
        sha2::Digest::update(&mut hasher, details.as_bytes());
        let hash = hex::encode(sha2::Digest::finalize(hasher));

        let _ = conn.execute(
            "INSERT INTO audit_logs (id, timestamp, actor, action, target, result, details, ip_address, prev_hash, hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![id, now_rfc, actor, action, target, result, details, "", prev_hash, hash],
        );
    }
}
