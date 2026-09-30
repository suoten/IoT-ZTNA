//! 管理面 JWT + RBAC 认证中间件
//!
//! 使用 HMAC-SHA256 实现 JWT 签发与验证，不依赖外部 JWT crate。

use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::sync::Arc;

type HmacSha256 = Hmac<Sha256>;

/// JWT Claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // 用户名
    pub role: String, // 角色: admin / operator / auditor
    pub exp: usize,   // 过期时间（unix timestamp）
    pub iat: usize,   // 签发时间
}

/// 用户角色枚举
#[derive(Debug, Clone, PartialEq)]
pub enum Role {
    Admin,    // 管理员：全部权限
    Operator, // 操作员：设备/策略/证书管理
    Auditor,  // 审计员：只读
}

impl Role {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "admin" => Some(Self::Admin),
            "operator" => Some(Self::Operator),
            "auditor" => Some(Self::Auditor),
            _ => None,
        }
    }

    pub fn can_write(&self) -> bool {
        matches!(self, Self::Admin | Self::Operator)
    }

    pub fn can_delete(&self) -> bool {
        matches!(self, Self::Admin)
    }

    pub fn can_read(&self) -> bool {
        true
    }
}

/// JWT 认证配置
#[derive(Clone)]
pub struct AuthConfig {
    pub secret: Vec<u8>,
    pub enabled: bool,
}

impl AuthConfig {
    pub fn new(secret: &str, enabled: bool) -> Self {
        Self {
            secret: secret.as_bytes().to_vec(),
            enabled,
        }
    }

    /// 签发 JWT token（HS256）
    pub fn sign_token(&self, username: &str, role: &str, hours: i64) -> Result<String, String> {
        let now = chrono::Utc::now().timestamp() as usize;
        let claims = Claims {
            sub: username.to_string(),
            role: role.to_string(),
            exp: now + (hours * 3600) as usize,
            iat: now,
        };

        let header = serde_json::json!({"alg": "HS256", "typ": "JWT"});
        let header_b64 = base64_url_encode(serde_json::to_string(&header).unwrap().as_bytes());
        let claims_b64 = base64_url_encode(serde_json::to_string(&claims).unwrap().as_bytes());
        let signing_input = format!("{}.{}", header_b64, claims_b64);

        let mut mac =
            Hmac::<Sha256>::new_from_slice(&self.secret).map_err(|e| format!("HMAC 密钥错误: {}", e))?;
        mac.update(signing_input.as_bytes());
        let signature = mac.finalize().into_bytes();
        let sig_b64 = base64_url_encode(&signature);

        Ok(format!("{}.{}", signing_input, sig_b64))
    }

    /// 验证 JWT token
    pub fn verify_token(&self, token: &str) -> Result<Claims, String> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err("JWT 格式无效".to_string());
        }

        let signing_input = format!("{}.{}", parts[0], parts[1]);

        // 验证签名
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.secret)
            .map_err(|e| format!("HMAC 密钥错误: {}", e))?;
        mac.update(signing_input.as_bytes());
        let expected_sig = mac.finalize().into_bytes();

        let provided_sig = base64_url_decode(parts[2]).map_err(|e| format!("签名解码失败: {}", e))?;

        if expected_sig.as_slice() != provided_sig.as_slice() {
            return Err("JWT 签名不匹配".to_string());
        }

        // 解析 Claims
        let claims_json = base64_url_decode(parts[1])
            .map_err(|e| format!("Claims 解码失败: {}", e))?;
        let claims: Claims = serde_json::from_slice(&claims_json)
            .map_err(|e| format!("Claims 解析失败: {}", e))?;

        // 检查过期
        let now = chrono::Utc::now().timestamp() as usize;
        if claims.exp < now {
            return Err("JWT 已过期".to_string());
        }

        Ok(claims)
    }
}

/// JWT 认证中间件
pub async fn jwt_auth_middleware(
    axum::extract::State(auth): axum::extract::State<Arc<AuthConfig>>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if !auth.enabled {
        return Ok(next.run(req).await);
    }

    // 非 API 请求（前端静态文件）不需要认证
    let path = req.uri().path();
    if !path.starts_with("/api/") && path != "/ws" {
        return Ok(next.run(req).await);
    }

    // API 中以下路径不需要认证
    if path.contains("/health")
        || path.contains("/metrics")
        || path.contains("/ws")
        || path.contains("/auth/login")
    {
        return Ok(next.run(req).await);
    }

    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let claims = auth
        .verify_token(token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // 将 claims 注入到 request extensions 中
    let mut req = req;
    req.extensions_mut().insert(claims);

    Ok(next.run(req).await)
}

/// 登录请求
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// 登录响应
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub expires_in: i64,
    pub role: String,
}

/// Base64 URL-safe 编码（无填充）
fn base64_url_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((n >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((n >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((n >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(n & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result.trim_end_matches('=').to_string()
}

/// Base64 URL-safe 解码
fn base64_url_decode(s: &str) -> Result<Vec<u8>, String> {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let s = s.trim_matches('=');
    let mut result = Vec::with_capacity(s.len() * 3 / 4);
    let bytes = s.as_bytes();

    for chunk in bytes.chunks(4) {
        let mut n = 0u32;
        let mut valid = 0;
        for &c in chunk {
            if let Some(idx) = CHARS.iter().position(|&x| x == c) {
                n = (n << 6) | idx as u32;
                valid += 1;
            } else {
                return Err(format!("无效 Base64 字符: {}", c as char));
            }
        }
        n <<= (4 - valid) * 6;
        if valid >= 2 {
            result.push((n >> 16) as u8);
        }
        if valid >= 3 {
            result.push((n >> 8) as u8);
        }
        if valid >= 4 {
            result.push(n as u8);
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jwt_sign_verify_roundtrip() {
        let config = AuthConfig::new("test-secret", true);
        let token = config.sign_token("admin", "admin", 1).unwrap();
        let claims = config.verify_token(&token).unwrap();
        assert_eq!(claims.sub, "admin");
        assert_eq!(claims.role, "admin");
    }

    #[test]
    fn jwt_invalid_token_rejected() {
        let config = AuthConfig::new("test-secret", true);
        assert!(config.verify_token("invalid.token.here").is_err());
    }

    #[test]
    fn jwt_wrong_secret_rejected() {
        let config1 = AuthConfig::new("secret1", true);
        let config2 = AuthConfig::new("secret2", true);
        let token = config1.sign_token("admin", "admin", 1).unwrap();
        assert!(config2.verify_token(&token).is_err());
    }
}
