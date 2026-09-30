//! IoT-ZTNA 智能层
//!
//! 包含:
//! - Autoencoder 行为基线异常检测（纯 Rust 实现，无需外部 ONNX 依赖）
//! - ONNX 模型管理（训练/热加载/版本管理/下发）
//! - SHAP-style 特征贡献度可解释性
//!
//! 架构:
//! ```text
//! 设备行为数据 ──► 特征提取 ──► Autoencoder ──► 重建误差 ──► 异常评分
//!                     │              │                          │
//!                     │              │                          ▼
//!                     │              │              SHAP 特征贡献度分解
//!                     │              │                          │
//!                     ▼              ▼                          ▼
//!               ONNX 模型管理    潜在空间              行为基线报告
//!               · 热加载         · 16维嵌入
//!               · 版本管理
//!               · 训练/下发
//! ```

pub mod autoencoder;
pub mod model_manager;
pub mod training;

pub use autoencoder::AutoencoderEngine;
pub use model_manager::ModelManager;
pub use training::ModelTrainer;
