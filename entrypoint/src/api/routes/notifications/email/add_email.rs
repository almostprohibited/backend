use common::db::{ServiceType, VerificationStatus};
use std::sync::Arc;

use crate::helpers::{get_token, is_email_valid};
use crate::routes::error_message_erasure::ApiError;
use crate::structs::ServerState;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::{Json, debug_handler};
use axum::{http::StatusCode, response::IntoResponse};
use axum_extra::extract::WithRejection;
use serde::Deserialize;
use serde_with::serde_as;
use tracing::{debug, error};

#[serde_as]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Payload {
    email: String,
}

#[debug_handler]
pub(crate) async fn add_email(
    headers: HeaderMap,
    State(state): State<Arc<ServerState>>,
    WithRejection(Json(payload), _): WithRejection<Json<Payload>, ApiError>,
) -> Result<impl IntoResponse, StatusCode> {
    let Some(token) = get_token(&headers) else {
        debug!("Missing token");

        return Ok(StatusCode::BAD_REQUEST.into_response());
    };

    let Some(active_session) = state.db.find_session(&token).await else {
        debug!("Invalid token");

        return Ok(StatusCode::BAD_REQUEST.into_response());
    };

    let email = payload.email.trim();

    if !is_email_valid(email) {
        error!("Request is missing valid email");

        return Ok(StatusCode::BAD_REQUEST.into_response());
    }

    for alert in state
        .db
        .get_notification_channels(active_session.user_id)
        .await
    {
        if alert.identifier == email && alert.user_id == active_session.user_id {
            error!("Channel already exists");

            return Ok(StatusCode::CONFLICT.into_response());
        }
    }

    state
        .db
        .create_notification_channel(
            email,
            active_session.user_id,
            ServiceType::Email,
            VerificationStatus::Pending,
        )
        .await;

    Ok(StatusCode::CREATED.into_response())
}
