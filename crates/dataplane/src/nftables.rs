//! nftables 数据面执行器（V2 — 实现生产级接口）
//!
//! 当 eBPF 不可用时（老内核 / 容器限制 / 非 Linux），降级到 nftables。
//! nftables 同样是内核态执行，性能高（≥ 3.13）。
//!
//! 在非 root 或 Windows 上，进一步降级为内存规则模式。

use crate::{DataPlaneExecutor, DataPlaneStats};
use async_trait::async_trait;
use iot_ztna_common::{Error, Result};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

/// nftables 执行器
pub struct NftablesExecutorV2 {
    table_name: String,
    /// 内存中的规则缓存（在非 root 环境或 Windows 上作为降级方案）
    rules: RwLock<HashMap<IpAddr, Vec<NftRule>>>,
    /// 统计计数器
    packets_passed: AtomicU64,
    packets_dropped: AtomicU64,
    bytes_passed: AtomicU64,
    /// 是否真正执行 nft 命令
    use_real_nft: bool,
}

/// nft 规则
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct NftRule {
    source_ip: IpAddr,
    target_port: u16,
    action: NftAction,
}

#[derive(Debug, Clone, Copy)]
enum NftAction {
    Accept,
    Drop,
}

impl NftablesExecutorV2 {
    pub fn new(table_name: &str) -> Self {
        let use_real_nft = cfg!(target_os = "linux") && std::process::Command::new("nft")
            .arg("--version")
            .output()
            .is_ok();

        let exec = Self {
            table_name: table_name.to_string(),
            rules: RwLock::new(HashMap::new()),
            packets_passed: AtomicU64::new(0),
            packets_dropped: AtomicU64::new(0),
            bytes_passed: AtomicU64::new(0),
            use_real_nft,
        };

        if use_real_nft {
            exec.init_table();
            tracing::info!("nftables 执行器已初始化（内核态）");
        } else {
            tracing::info!("nft 不可用，使用内存规则降级模式");
        }

        exec
    }

    /// 初始化 nftables 表
    fn init_table(&self) {
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
impl DataPlaneExecutor for NftablesExecutorV2 {
    async fn allow(&self, device_ip: IpAddr, target_port: u16) -> Result<()> {
        let rule = NftRule {
            source_ip: device_ip,
            target_port,
            action: NftAction::Accept,
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

        self.packets_passed.fetch_add(1, Ordering::Relaxed);

        let mut rules = self.rules.write().map_err(|e| {
            Error::Internal(format!("获取规则写锁失败: {}", e))
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

        self.packets_dropped.fetch_add(1, Ordering::Relaxed);

        let mut rules = self.rules.write().map_err(|e| {
            Error::Internal(format!("获取规则写锁失败: {}", e))
        })?;
        rules.insert(
            device_ip,
            vec![NftRule {
                source_ip: device_ip,
                target_port: 0,
                action: NftAction::Drop,
            }],
        );
        Ok(())
    }

    async fn clear(&self, device_ip: IpAddr) -> Result<()> {
        let mut rules = self.rules.write().map_err(|e| {
            Error::Internal(format!("获取规则写锁失败: {}", e))
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
            Error::Internal(format!("获取规则写锁失败: {}", e))
        })?;
        rules.clear();
        Ok(())
    }

    async fn rule_count(&self) -> Result<usize> {
        let rules = self.rules.read().map_err(|e| {
            Error::Internal(format!("获取规则读锁失败: {}", e))
        })?;
        Ok(rules.values().map(|v| v.len()).sum())
    }

    async fn stats(&self) -> Result<DataPlaneStats> {
        let rules = self.rules.read().map_err(|e| {
            Error::Internal(format!("获取规则读锁失败: {}", e))
        })?;
        let active_rules = rules.values().map(|v| v.len()).sum();

        Ok(DataPlaneStats {
            mode: if self.use_real_nft { "nftables" } else { "memory" }.to_string(),
            active_rules,
            packets_passed: self.packets_passed.load(Ordering::Relaxed),
            packets_dropped: self.packets_dropped.load(Ordering::Relaxed),
            bytes_passed: self.bytes_passed.load(Ordering::Relaxed),
        })
    }

    fn mode(&self) -> &str {
        if self.use_real_nft { "nftables" } else { "memory" }
    }
}
