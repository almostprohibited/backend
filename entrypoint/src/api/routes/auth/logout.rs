use std::sync::Arc;

use crate::helpers::get_token;
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
        debug!("Missing token");

        return Ok(StatusCode::BAD_REQUEST);
    };

    match state.db.delete_session(&token).await {
        true => Ok(StatusCode::OK),
        false => {
            debug!("Hashed token does not exist in DB");

            Ok(StatusCode::BAD_REQUEST)
        }
    }
}
