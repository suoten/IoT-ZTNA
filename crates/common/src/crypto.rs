//! 敏感配置加密（AES-256-GCM）
//!
//! 用于加密存储中的敏感字段：API 密钥、数据库密码、JWT 密钥等。
//! 使用 ChaCha20-Poly1305（ring 原生支持，无额外依赖）。

use ring::aead;
use ring::rand::{SecureRandom, SystemRandom};
use std::fmt;

/// 加密密钥（256 位）
pub struct EncryptionKey {
    key: Vec<u8>,
}

impl EncryptionKey {
    /// 从环境变量或字符串创建密钥
    pub fn from_secret(secret: &str) -> Self {
        // 使用 SHA-256 派生固定长度密钥
        use ring::digest;
        let digest = digest::digest(&digest::SHA256, secret.as_bytes());
        Self {
            key: digest.as_ref().to_vec(),
        }
    }

    /// 生成随机密钥
    pub fn generate() -> Self {
        let rng = SystemRandom::new();
        let mut key = vec![0u8; 32];
        rng.fill(&mut key).expect("生成随机密钥失败");
        Self { key }
    }
}

impl Clone for EncryptionKey {
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
        }
    }
}

impl fmt::Debug for EncryptionKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EncryptionKey([REDACTED])")
    }
}

/// 加密器
pub struct ConfigEncryptor {
    key: EncryptionKey,
}

impl ConfigEncryptor {
    pub fn new(key: EncryptionKey) -> Self {
        Self { key }
    }

    /// 加密明文，返回 Base64 编码的密文
    pub fn encrypt(&self, plaintext: &str) -> Result<String, CryptoError> {
        let rng = SystemRandom::new();
        let mut nonce = vec![0u8; 12]; // 96-bit nonce for ChaCha20-Poly1305
        rng.fill(&mut nonce)
            .map_err(|_| CryptoError::NonceGeneration)?;

        let key = aead::LessSafeKey::new(
            aead::UnboundKey::new(&aead::CHACHA20_POLY1305, &self.key.key)
                .map_err(|_| CryptoError::InvalidKey)?,
        );

        let mut ciphertext = plaintext.as_bytes().to_vec();

        let tag = key.seal_in_place_separate_tag(
            aead::Nonce::try_assume_unique_for_key(&nonce)
                .map_err(|_| CryptoError::NonceGeneration)?,
            aead::Aad::empty(),
            &mut ciphertext,
        )
        .map_err(|_| CryptoError::EncryptionFailed)?;

        // nonce (12) || ciphertext || tag (16)
        let mut result = nonce.clone();
        result.extend_from_slice(&ciphertext);
        result.extend_from_slice(tag.as_ref());
        Ok(base64_encode(&result))
    }

    /// 解密 Base64 编码的密文
    pub fn decrypt(&self, encrypted: &str) -> Result<String, CryptoError> {
        let data = base64_decode(encrypted)?;
        if data.len() < 12 + 16 {
            return Err(CryptoError::DataTooShort);
        }

        let nonce = &data[..12];
        let ciphertext = &data[12..];

        let key = aead::LessSafeKey::new(
            aead::UnboundKey::new(&aead::CHACHA20_POLY1305, &self.key.key)
                .map_err(|_| CryptoError::InvalidKey)?,
        );

        let mut ciphertext = ciphertext.to_vec();
        let plaintext = key
            .open_in_place(
                aead::Nonce::try_assume_unique_for_key(nonce)
                    .map_err(|_| CryptoError::NonceGeneration)?,
                aead::Aad::empty(),
                &mut ciphertext,
            )
            .map_err(|_| CryptoError::DecryptionFailed)?;

        String::from_utf8(plaintext.to_vec()).map_err(|_| CryptoError::InvalidUtf8)
    }
}

/// 加密错误
#[derive(Debug, Clone)]
pub enum CryptoError {
    NonceGeneration,
    InvalidKey,
    EncryptionFailed,
    DecryptionFailed,
    DataTooShort,
    InvalidUtf8,
    Base64Error,
}

impl fmt::Display for CryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CryptoError::NonceGeneration => write!(f, "nonce 生成失败"),
            CryptoError::InvalidKey => write!(f, "无效密钥"),
            CryptoError::EncryptionFailed => write!(f, "加密失败"),
            CryptoError::DecryptionFailed => write!(f, "解密失败"),
            CryptoError::DataTooShort => write!(f, "密文数据过短"),
            CryptoError::InvalidUtf8 => write!(f, "解密后数据非有效 UTF-8"),
            CryptoError::Base64Error => write!(f, "Base64 编解码错误"),
        }
    }
}

impl std::error::Error for CryptoError {}

/// 简单 Base64 编码（不依赖外部 crate）
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
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
    result
}

/// 简单 Base64 解码
fn base64_decode(s: &str) -> Result<Vec<u8>, CryptoError> {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
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
                return Err(CryptoError::Base64Error);
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
    fn encrypt_decrypt_roundtrip() {
        let key = EncryptionKey::from_secret("test-secret-key");
        let encryptor = ConfigEncryptor::new(key);
        let plaintext = "sensitive-config-value-12345";
        let encrypted = encryptor.encrypt(plaintext).unwrap();
        assert_ne!(encrypted, plaintext);
        let decrypted = encryptor.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }
}
