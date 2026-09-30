//! IoT-ZTNA API 后端
//!
//! 基于 Axum 的 REST API + WebSocket 实时推送。

pub mod error;
pub mod routes;
pub mod state;
pub mod ws;
pub mod auth;

pub use routes::create_router;
pub use state::AppState;
pub use error::{AppError, AppResult};
