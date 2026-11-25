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

    pub fn upstream(field: &'static str, error: anyhow::Error) -> Self {
        tracing::warn!(error = %format!("{error:#}"), "a platform call failed");
        Self::field(field, public_message(&error.to_string()))
    }
}

const PLATFORM_FAILED: &str =
    "Er ging iets mis bij het platform. Probeer het later opnieuw of neem contact op met support.";

const CUSTOMER_SAFE: &[&str] = &[
    "Stacklane werd herstart tijdens het leveren. Probeer het opnieuw.",
    "Stacklane werd herstart tijdens het importeren.",
    "De server van dit volume is opgeheven.",
    "dit bestand is te groot om hier te openen",
];

pub fn public_message(message: &str) -> String {
    if CUSTOMER_SAFE.contains(&message) {
        message.to_owned()
    } else {
        PLATFORM_FAILED.to_owned()
    }
}

pub fn public_error<S: serde::Serializer>(
    value: &Option<String>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match value {
        Some(message) => serializer.serialize_some(&tr(&public_message(message))),
        None => serializer.serialize_none(),
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

#[cfg(test)]
mod tests {
    use super::public_message;

    #[test]
    fn hides_platform_details_from_customers() {
        assert_eq!(
            public_message(
                "Proxmox antwoordde 500 Internal Server Error: storage 'ceph-int' is not online"
            ),
            super::PLATFORM_FAILED
        );
        assert_eq!(
            public_message("error sending request for url (https://10.0.0.5:8006/api2/json/nodes)"),
            super::PLATFORM_FAILED
        );
        assert_eq!(
            public_message("Proxmox-taak mislukt: unable to create VM 105 - storage 'ceph-int' is not online"),
            super::PLATFORM_FAILED
        );
        assert_eq!(
            public_message("De server van dit volume is opgeheven."),
            "De server van dit volume is opgeheven."
        );
    }
}
