use std::collections::BTreeMap;

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("unauthenticated")]
    Unauthenticated,
    #[error("not found")]
    NotFound,
    #[error("validation failed")]
    Validation(BTreeMap<&'static str, String>),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn field(field: &'static str, message: impl Into<String>) -> Self {
        Self::Validation(BTreeMap::from([(field, message.into())]))
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            Self::Unauthenticated => (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "message": "Je bent niet ingelogd." })),
            )
                .into_response(),
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                Json(json!({ "message": "Niet gevonden." })),
            )
                .into_response(),
            Self::Validation(errors) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!({ "message": "Controleer de ingevulde gegevens.", "errors": errors })),
            )
                .into_response(),
            Self::Database(error) => {
                tracing::error!(?error, "database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "message": "Er ging iets mis." })),
                )
                    .into_response()
            }
            Self::Internal(error) => {
                tracing::error!(?error, "internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "message": "Er ging iets mis." })),
                )
                    .into_response()
            }
        }
    }
}
