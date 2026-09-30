//! IoT-ZTNA OPA/Rego 策略引擎
//!
//! 集成 OPA (Open Policy Agent) 策略评估能力：
//! - Rego 策略解析与评估
//! - Bundle 热加载（无需重启）
//! - 本地评估（无需外部 OPA 进程）
//! - 策略模拟（dry-run）
//!
//! 架构:
//! ```text
//! ┌─ 策略生命周期 ──────────────────────────────────────────────┐
//! │                                                             │
//! │  Rego 编写 → 策略模拟(dry-run) → 管理员审批 → Bundle 打包  │
//! │                                                             │
//! │  → 分发到边缘 → 热加载生效 → 全操作留痕 → 版本回滚        │
//! │                                                             │
//! └─────────────────────────────────────────────────────────────┘
//! ```

pub mod engine;
pub mod bundle;

pub use engine::RegoEngine;
pub use bundle::BundleManager;
