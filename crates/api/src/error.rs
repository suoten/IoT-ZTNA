//! 错误处理 - 将内部 Error 转为 Axum 响应

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use iot_ztna_common::{ApiResponse, Error};
use serde::Serialize;

/// 应用错误，实现 IntoResponse
pub struct AppError(pub Error);

impl From<Error> for AppError {
    fn from(e: Error) -> Self {
        AppError(e)
    }
}

/// 用于错误响应的空数据类型
#[derive(Serialize)]
struct EmptyData;

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.0.status_code())
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let body = ApiResponse::<EmptyData>::err(self.0.error_code(), &self.0.to_string());
        (status, Json(body)).into_response()
    }
}

pub type AppResult<T> = std::result::Result<Json<ApiResponse<T>>, AppError>;
