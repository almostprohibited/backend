use std::sync::Arc;

use crate::helpers::get_token;
use crate::routes::error_message_erasure::ApiError;
use crate::structs::ServerState;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::{Json, debug_handler};
use axum::{http::StatusCode, response::IntoResponse};
use axum_extra::extract::WithRejection;
use serde::Deserialize;
use serde_with::serde_as;
use tracing::debug;

#[serde_as]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Payload {
    identifiers: Vec<String>,
}

#[debug_handler]
pub(crate) async fn delete_channels(
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

    let handles: Vec<_> = payload
        .identifiers
        .iter()
        .map(|identifier| {
            state
                .db
                .delete_notification_channel(active_session.user_id, identifier)
        })
        .collect();

    for handle in handles {
        handle.await;
    }

    // TODO: handle if one of the deletes fails
    Ok(StatusCode::OK.into_response())
}
