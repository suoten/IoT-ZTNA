//! IoT-ZTNA 数据面
//!
//! 提供内核态流量执行能力：
//! - eBPF/XDP 程序加载与策略管理（Linux 内核 ≥ 5.10）
//! - nftables 降级执行（兼容老内核/容器/非 Linux 环境）
//! - BPF Map 策略同步
//! - 统计采集与上报
//!
//! 架构:
//! ```text
//! ┌─ 内核态 ────────────────────────────────────┐
//! │  XDP 程序 (网络驱动层)                        │
//! │  · 包到达即决策（pass/drop/redirect）         │
//! │  · 策略查表（BPF Map）                        │
//! │  · 延迟：~10-30μs                             │
//! │                                               │
//! │  BPF Map（共享数据）                          │
//! │  · 策略表：IP → allow/deny 规则               │
//! │  · 统计表：包计数/字节数/拦截数                │
//! └───────────────────────────────────────────────┘
//! ┌─ 用户态 ────────────────────────────────────┐
//! │  DataPlaneManager (Rust)                     │
//! │  · 加载/管理 BPF 程序                         │
//! │  · 策略更新（BPF Map 写入）                   │
//! │  · 统计采集与上报                              │
//! │  · nftables 降级                              │
//! └───────────────────────────────────────────────┘
//! ```

pub mod ebpf;
pub mod nftables;

use iot_ztna_common::{Result, PolicyDecision};
use std::net::IpAddr;
use std::sync::Arc;
use async_trait::async_trait;

/// 数据面执行器 trait
#[async_trait]
pub trait DataPlaneExecutor: Send + Sync {
    /// 应用允许规则：允许指定 IP 访问指定端口
    async fn allow(&self, device_ip: IpAddr, target_port: u16) -> Result<()>;

    /// 应用拒绝规则：拒绝指定 IP 的所有流量
    async fn deny(&self, device_ip: IpAddr) -> Result<()>;

    /// 清除设备所有规则
    async fn clear(&self, device_ip: IpAddr) -> Result<()>;

    /// 清除所有规则
    async fn clear_all(&self) -> Result<()>;

    /// 获取规则数
    async fn rule_count(&self) -> Result<usize>;

    /// 获取数据面统计信息
    async fn stats(&self) -> Result<DataPlaneStats>;

    /// 获取当前执行模式
    fn mode(&self) -> &str;
}

/// 数据面统计信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct DataPlaneStats {
    /// 执行模式: "ebpf" | "nftables" | "memory"
    pub mode: String,
    /// 活跃规则数
    pub active_rules: usize,
    /// 累计通过包数
    pub packets_passed: u64,
    /// 累计拦截包数
    pub packets_dropped: u64,
    /// 累计通过字节数
    pub bytes_passed: u64,
}

impl Default for DataPlaneStats {
    fn default() -> Self {
        Self {
            mode: "unknown".to_string(),
            active_rules: 0,
            packets_passed: 0,
            packets_dropped: 0,
            bytes_passed: 0,
        }
    }
}

/// 数据面管理器
///
/// 自动选择最佳执行后端：
/// 1. 优先使用 eBPF/XDP（内核 ≥ 5.10 且有 CAP_BPF）
/// 2. 降级使用 nftables（内核 ≥ 3.13）
/// 3. 最终降级使用内存规则（非 root / 非 Linux）
pub struct DataPlaneManager {
    executor: Arc<dyn DataPlaneExecutor>,
    /// 是否使用 eBPF
    use_ebpf: bool,
}

impl DataPlaneManager {
    /// 创建数据面管理器，自动选择最佳后端
    pub fn new(table_name: &str) -> Self {
        // 尝试 eBPF
        let (executor, use_ebpf) = match ebpf::EbpfExecutor::try_new(table_name) {
            Some(ebpf_exec) => {
                tracing::info!("✅ eBPF/XDP 数据面已初始化");
                (Arc::new(ebpf_exec) as Arc<dyn DataPlaneExecutor>, true)
            }
            None => {
                tracing::info!("⚠️ eBPF 不可用，降级到 nftables");
                let nft = nftables::NftablesExecutorV2::new(table_name);
                (Arc::new(nft) as Arc<dyn DataPlaneExecutor>, false)
            }
        };

        Self { executor, use_ebpf }
    }

    /// 获取执行器引用
    pub fn executor(&self) -> &Arc<dyn DataPlaneExecutor> {
        &self.executor
    }

    /// 是否使用 eBPF
    pub fn is_ebpf(&self) -> bool {
        self.use_ebpf
    }

    /// 获取当前模式
    pub fn mode(&self) -> &str {
        self.executor.mode()
    }
}
