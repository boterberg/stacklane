use axum::Json;
use axum::extract::State;

use crate::auth::CurrentAdmin;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::updates::{Overview, compare};

pub async fn show(State(state): State<AppState>, _admin: CurrentAdmin) -> Json<Overview> {
    Json(state.updates.overview())
}

pub async fn check(
    State(state): State<AppState>,
    _admin: CurrentAdmin,
) -> AppResult<Json<Overview>> {
    state.updates.check().await.map_err(AppError::Internal)?;
    Ok(Json(state.updates.overview()))
}

pub async fn start(
    State(state): State<AppState>,
    admin: CurrentAdmin,
) -> AppResult<Json<Overview>> {
    if state.config.demo_mode {
        return Err(AppError::field("update", "Bijwerken kan niet in de demo."));
    }
    let overview = state.updates.overview();
    if !overview.can_update {
        return Err(AppError::field(
            "update",
            "Bijwerken vanuit het panel is op deze server niet ingesteld.",
        ));
    }
    if overview
        .progress
        .as_ref()
        .is_some_and(|progress| progress.is_running())
    {
        return Err(AppError::field("update", "Er loopt al een update."));
    }
    let Some(target) = overview
        .latest
        .filter(|latest| compare(latest, overview.current).is_gt())
    else {
        return Err(AppError::field("update", "Er is geen nieuwere versie."));
    };
    state.updates.request(&target).map_err(AppError::Internal)?;
    tracing::info!(admin = %admin.id, %target, "update requested from the panel");
    Ok(Json(state.updates.overview()))
}
