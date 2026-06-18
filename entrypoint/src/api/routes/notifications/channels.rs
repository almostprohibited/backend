use std::sync::Arc;

use crate::helpers::get_token;
use crate::structs::ServerState;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::{Json, debug_handler};
use axum::{http::StatusCode, response::IntoResponse};
use common::db::VerificationStatus;
use serde::Serialize;
use tracing::debug;

#[derive(Serialize, Debug)]
struct Output {
    identifier: String,
    status: VerificationStatus,
}

#[debug_handler]
pub(crate) async fn get_notification_channels(
    headers: HeaderMap,
    State(state): State<Arc<ServerState>>,
) -> Result<impl IntoResponse, StatusCode> {
    let Some(token) = get_token(&headers) else {
        debug!("Missing token");

        return Ok(StatusCode::BAD_REQUEST.into_response());
    };

    let Some(active_session) = state.db.find_session(&token).await else {
        debug!("Invalid token");

        return Ok(StatusCode::BAD_REQUEST.into_response());
    };

    let notification_channels: Vec<Output> = state
        .db
        .get_notification_channels(active_session.user_id)
        .await
        .iter()
        .map(|channel| Output {
            identifier: channel.identifier.clone(),
            status: channel.status.clone(),
        })
        .collect();

    debug!("{:?}", notification_channels);

    let response: Json<Vec<Output>> = Json::from(notification_channels);

    Ok(response.into_response())
}
