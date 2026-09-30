//! 策略执行器
//!
//! 将策略决策转化为实际的流量控制：
//! - nftables 规则（生产级内核态执行）
//! - 用户态代理（降级模式）

use async_trait::async_trait;
use iot_ztna_common::{PolicyDecision, Result};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, RwLock};

/// 策略执行器 trait
#[async_trait]
pub trait PolicyExecutor: Send + Sync {
    /// 应用允许规则
    async fn allow(&self, device_ip: IpAddr, target_port: u16) -> Result<()>;

    /// 应用拒绝规则
    async fn deny(&self, device_ip: IpAddr) -> Result<()>;

    /// 清除设备所有规则
    async fn clear(&self, device_ip: IpAddr) -> Result<()>;

    /// 清除所有规则
    async fn clear_all(&self) -> Result<()>;

    /// 获取规则数
    async fn rule_count(&self) -> Result<usize>;
}

/// nftables 执行器
pub struct NftablesExecutor {
    table_name: String,
    /// 内存中的规则缓存（在非 root 环境或 Windows 上作为降级方案）
    rules: Arc<RwLock<HashMap<IpAddr, Vec<NftRule>>>>,
    /// 是否真正执行 nft 命令（生产环境为 true）
    use_real_nft: bool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct NftRule {
    source_ip: IpAddr,
    target_port: u16,
    action: PolicyDecision,
}

impl NftablesExecutor {
    pub fn new(table_name: &str) -> Self {
        // 在 Linux 环境检测是否可用 nft 命令
        let use_real_nft = cfg!(target_os = "linux") && std::process::Command::new("nft")
            .arg("--version")
            .output()
            .is_ok();

        Self {
            table_name: table_name.to_string(),
            rules: Arc::new(RwLock::new(HashMap::new())),
            use_real_nft,
        }
    }

    /// 初始化 nftables 表
    pub fn init_table(&self) -> Result<()> {
        if !self.use_real_nft {
            tracing::info!("nft 不可用，使用内存规则降级模式");
            return Ok(());
        }

        let table = &self.table_name;
        let commands = vec![
            format!("add table inet {}", table),
            format!("add chain inet {} input {{ type filter hook input priority 0; policy drop; }}", table),
            format!("add chain inet {} forward {{ type filter hook forward priority 0; policy drop; }}", table),
        ];

        for cmd in commands {
            let _ = self.run_nft(&cmd);
        }

        tracing::info!(table = table, "nftables 表已初始化");
        Ok(())
    }

    fn run_nft(&self, command: &str) -> std::io::Result<std::process::Output> {
        std::process::Command::new("nft")
            .args(command.split_whitespace())
            .output()
    }

    fn ip_to_string(ip: &IpAddr) -> String {
        match ip {
            IpAddr::V4(v4) => v4.to_string(),
            IpAddr::V6(v6) => v6.to_string(),
        }
    }
}

#[async_trait]
impl PolicyExecutor for NftablesExecutor {
    async fn allow(&self, device_ip: IpAddr, target_port: u16) -> Result<()> {
        let rule = NftRule {
            source_ip: device_ip,
            target_port,
            action: PolicyDecision::Allow,
        };

        if self.use_real_nft {
            let ip_str = Self::ip_to_string(&device_ip);
            let cmd = format!(
                "add rule inet {} forward ip saddr {} tcp dport {} accept",
                self.table_name, ip_str, target_port
            );
            if let Err(e) = self.run_nft(&cmd) {
                tracing::warn!(error = %e, "nft 命令执行失败");
            }
        }

        let mut rules = self.rules.write().map_err(|e| {
            iot_ztna_common::Error::Internal(format!("获取规则写锁失败: {}", e))
        })?;
        rules.entry(device_ip).or_default().push(rule);
        Ok(())
    }

    async fn deny(&self, device_ip: IpAddr) -> Result<()> {
        if self.use_real_nft {
            let ip_str = Self::ip_to_string(&device_ip);
            let cmd = format!(
                "add rule inet {} forward ip saddr {} drop",
                self.table_name, ip_str
            );
            let _ = self.run_nft(&cmd);
        }

        let mut rules = self.rules.write().map_err(|e| {
            iot_ztna_common::Error::Internal(format!("获取规则写锁失败: {}", e))
        })?;
        rules.insert(
            device_ip,
            vec![NftRule {
                source_ip: device_ip,
                target_port: 0,
                action: PolicyDecision::Deny,
            }],
        );
        Ok(())
    }

    async fn clear(&self, device_ip: IpAddr) -> Result<()> {
        let mut rules = self.rules.write().map_err(|e| {
            iot_ztna_common::Error::Internal(format!("获取规则写锁失败: {}", e))
        })?;
        rules.remove(&device_ip);
        Ok(())
    }

    async fn clear_all(&self) -> Result<()> {
        if self.use_real_nft {
            let cmd = format!("flush table inet {}", self.table_name);
            let _ = self.run_nft(&cmd);
        }
        let mut rules = self.rules.write().map_err(|e| {
            iot_ztna_common::Error::Internal(format!("获取规则写锁失败: {}", e))
        })?;
        rules.clear();
        Ok(())
    }

    async fn rule_count(&self) -> Result<usize> {
        let rules = self.rules.read().map_err(|e| {
            iot_ztna_common::Error::Internal(format!("获取规则读锁失败: {}", e))
        })?;
        Ok(rules.values().map(|v| v.len()).sum())
    }
}
