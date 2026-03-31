use std::sync::Arc;

use crate::helpers::{get_token, hash_string};
use crate::structs::ServerState;

use axum::debug_handler;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::{http::StatusCode, response::IntoResponse};
use tracing::debug;

#[debug_handler]
pub(crate) async fn logout(
    headers: HeaderMap,
    State(state): State<Arc<ServerState>>,
) -> Result<impl IntoResponse, StatusCode> {
    let Some(token) = get_token(&headers) else {
        debug!("{headers:?}");
        debug!("Missing token");

        return Ok(StatusCode::BAD_REQUEST);
    };

    let hashed_token = hash_string(&token);

    match state.db.delete_session(&hashed_token).await {
        true => Ok(StatusCode::OK),
        false => {
            debug!("Hashed token does not exist in DB");

            Ok(StatusCode::BAD_REQUEST)
        }
    }
}
