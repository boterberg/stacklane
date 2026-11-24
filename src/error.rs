use std::collections::BTreeMap;

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

use crate::i18n::tr;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("unauthenticated")]
    Unauthenticated,
    #[error("forbidden")]
    Forbidden,
    #[error("not found")]
    NotFound,
    #[error("too many requests")]
    TooMany,
    #[error("busy")]
    Busy,
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
                Json(json!({ "message": tr("Je bent niet ingelogd.") })),
            )
                .into_response(),
            Self::Forbidden => (
                StatusCode::FORBIDDEN,
                Json(json!({ "message": tr("Je hebt geen toegang tot dit onderdeel.") })),
            )
                .into_response(),
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                Json(json!({ "message": tr("Niet gevonden.") })),
            )
                .into_response(),
            Self::TooMany => (
                StatusCode::TOO_MANY_REQUESTS,
                [(axum::http::header::RETRY_AFTER, "60")],
                Json(json!({ "message": tr("Te veel pogingen. Probeer het over een minuut opnieuw.") })),
            )
                .into_response(),
            Self::Busy => (
                StatusCode::TOO_MANY_REQUESTS,
                [(axum::http::header::RETRY_AFTER, "5")],
                Json(json!({ "message": tr("Het is even te druk. Probeer het over een paar seconden opnieuw.") })),
            )
                .into_response(),
            Self::Validation(errors) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!({ "message": tr("Controleer de ingevulde gegevens."), "errors": errors.into_iter().map(|(field, message)| (field, tr(&message))).collect::<BTreeMap<_, _>>() })),
            )
                .into_response(),
            Self::Database(error) => {
                tracing::error!(?error, "database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "message": tr("Er ging iets mis.") })),
                )
                    .into_response()
            }
            Self::Internal(error) => {
                tracing::error!(?error, "internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "message": tr("Er ging iets mis.") })),
                )
                    .into_response()
            }
        }
    }
}
