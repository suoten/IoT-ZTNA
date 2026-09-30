//! 证书签发机构 (CA)
//!
//! 基于 rcgen 实现自签名 Root CA，用于签发 SVID。

use chrono::{DateTime, Duration, Utc};
use iot_ztna_common::{Error, Result, SpiffeId};
use rcgen::{
    CertificateParams, DistinguishedName, DnType, IsCa, KeyIdMethod, KeyPair, KeyUsagePurpose,
    SanType,
};

/// 证书签发机构
#[derive(Clone)]
pub struct CertificateAuthority {
    /// Root CA 证书 PEM
    pub root_cert_pem: String,
    /// Root CA 私钥 PEM
    pub root_key_pem: String,
    /// 信任域
    pub trust_domain: String,
}

impl CertificateAuthority {
    /// 生成新的 Root CA
    pub fn generate(trust_domain: &str) -> Result<Self> {
        let mut params = CertificateParams::default();
        params.distinguished_name = DistinguishedName::new();
        params
            .distinguished_name
            .push(DnType::CommonName, format!("IoT-ZTNA Root CA ({})", trust_domain));
        params
            .distinguished_name
            .push(DnType::OrganizationName, "IoT-ZTNA");
        params.is_ca = IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        params.key_usages = vec![
            KeyUsagePurpose::KeyCertSign,
            KeyUsagePurpose::CrlSign,
        ];
        params.key_identifier_method = KeyIdMethod::Sha256;
        // Root CA 有效期 10 年
        params.not_before = rcgen::date_time_ymd(2026, 1, 1);
        params.not_after = rcgen::date_time_ymd(2036, 1, 1);

        let key_pair = KeyPair::generate()
            .map_err(|e| Error::Certificate(format!("生成密钥对失败: {}", e)))?;
        let cert = params
            .self_signed(&key_pair)
            .map_err(|e| Error::Certificate(format!("自签名 Root CA 失败: {}", e)))?;

        let root_cert_pem = cert.pem();
        let root_key_pem = key_pair.serialize_pem();

        Ok(Self {
            root_cert_pem,
            root_key_pem,
            trust_domain: trust_domain.to_string(),
        })
    }

    /// 从 PEM 恢复 CA
    pub fn from_pem(root_cert_pem: &str, root_key_pem: &str, trust_domain: &str) -> Self {
        Self {
            root_cert_pem: root_cert_pem.to_string(),
            root_key_pem: root_key_pem.to_string(),
            trust_domain: trust_domain.to_string(),
        }
    }

    /// 签发设备 SVID 证书
    pub fn issue_svid(
        &self,
        spiffe_id: &SpiffeId,
        ttl_days: u32,
    ) -> Result<(String, String, DateTime<Utc>)> {
        // 构建设备证书参数
        let mut params = CertificateParams::default();
        params.distinguished_name = DistinguishedName::new();
        params
            .distinguished_name
            .push(DnType::CommonName, &spiffe_id.name);
        params
            .distinguished_name
            .push(DnType::OrganizationName, "IoT-ZTNA");

        // SPIFFE URI SAN
        let spiffe_uri = spiffe_id.to_string();
        let uri: rcgen::Ia5String = spiffe_uri
            .try_into()
            .map_err(|e| Error::Certificate(format!("URI 转换失败: {:?}", e)))?;
        params.subject_alt_names = vec![SanType::URI(uri)];

        params.key_usages = vec![
            KeyUsagePurpose::DigitalSignature,
            KeyUsagePurpose::KeyEncipherment,
        ];
        params.key_identifier_method = KeyIdMethod::Sha256;

        let not_before = Utc::now();
        let not_after = Utc::now() + Duration::days(ttl_days as i64);

        // 使用 rcgen 的 date_time_ymd 来设置时间（避免直接依赖 time crate）
        let nb = rcgen::date_time_ymd(
            not_before.format("%Y").to_string().parse().unwrap_or(2026),
            not_before.format("%m").to_string().parse().unwrap_or(1),
            not_before.format("%d").to_string().parse().unwrap_or(1),
        );
        let na = rcgen::date_time_ymd(
            not_after.format("%Y").to_string().parse().unwrap_or(2027),
            not_after.format("%m").to_string().parse().unwrap_or(1),
            not_after.format("%d").to_string().parse().unwrap_or(1),
        );
        params.not_before = nb;
        params.not_after = na;

        // 生成设备密钥对
        let device_key_pair = KeyPair::generate()
            .map_err(|e| Error::Certificate(format!("生成设备密钥对失败: {}", e)))?;

        // 使用 Root CA 签发设备证书
        let root_key = KeyPair::from_pem(&self.root_key_pem)
            .map_err(|e| Error::Certificate(format!("解析 Root CA 私钥失败: {}", e)))?;
        let root_params = CertificateParams::from_ca_cert_pem(&self.root_cert_pem)
            .map_err(|e| Error::Certificate(format!("解析 Root CA 证书失败: {}", e)))?;
        let root_cert = root_params
            .self_signed(&root_key)
            .map_err(|e| Error::Certificate(format!("重建 Root CA 失败: {}", e)))?;

        let device_cert = CertificateParams::signed_by(
            params,
            &device_key_pair,
            &root_cert,
            &root_key,
        )
        .map_err(|e| Error::Certificate(format!("签发 SVID 失败: {}", e)))?;

        let cert_pem = device_cert.pem();
        let key_pem = device_key_pair.serialize_pem();

        Ok((cert_pem, key_pem, not_after))
    }

    /// 获取信任束 (trust bundle)
    pub fn trust_bundle_pem(&self) -> String {
        self.root_cert_pem.clone()
    }

    /// 持久化 CA 到目录
    pub fn save_to_dir(&self, dir: &std::path::Path) -> Result<()> {
        std::fs::create_dir_all(dir).map_err(|e| Error::Io(e))?;
        std::fs::write(dir.join("ca.crt"), &self.root_cert_pem).map_err(|e| Error::Io(e))?;
        std::fs::write(dir.join("ca.key"), &self.root_key_pem).map_err(|e| Error::Io(e))?;
        Ok(())
    }

    /// 从目录加载 CA
    pub fn load_from_dir(dir: &std::path::Path, trust_domain: &str) -> Result<Self> {
        let cert_pem = std::fs::read_to_string(dir.join("ca.crt"))
            .map_err(|e| Error::Certificate(format!("读取 CA 证书失败: {}", e)))?;
        let key_pem = std::fs::read_to_string(dir.join("ca.key"))
            .map_err(|e| Error::Certificate(format!("读取 CA 私钥失败: {}", e)))?;
        Ok(Self::from_pem(&cert_pem, &key_pem, trust_domain))
    }

    /// 加载或生成 CA
    pub fn load_or_generate(dir: &std::path::Path, trust_domain: &str) -> Result<Self> {
        let cert_path = dir.join("ca.crt");
        if cert_path.exists() {
            tracing::info!("从 {} 加载已有 CA", dir.display());
            Self::load_from_dir(dir, trust_domain)
        } else {
            tracing::info!("生成新的 Root CA, 信任域: {}", trust_domain);
            let ca = Self::generate(trust_domain)?;
            ca.save_to_dir(dir)?;
            tracing::info!("Root CA 已保存到 {}", dir.display());
            Ok(ca)
        }
    }
}
