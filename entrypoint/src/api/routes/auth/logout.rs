use crate::constants::TOKEN_COOKIE_NAME;
use std::sync::Arc;

use crate::helpers::get_token;
use crate::structs::ServerState;

use axum::debug_handler;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue};
use axum::{http::StatusCode, response::IntoResponse};
use tracing::debug;

#[debug_handler]
pub(crate) async fn logout(
    headers: HeaderMap,
    State(state): State<Arc<ServerState>>,
) -> Result<impl IntoResponse, StatusCode> {
    let Some(token) = get_token(&headers) else {
        debug!("Missing token");

        return Ok(StatusCode::BAD_REQUEST.into_response());
    };

    let mut return_headers = HeaderMap::new();
    return_headers.append(
        "Set-Cookie",
        HeaderValue::from_str(&format!("{TOKEN_COOKIE_NAME}=\"\"; expires")).unwrap(),
    );

    let response_code = match state.db.delete_session(&token).await {
        true => StatusCode::OK,
        false => {
            debug!("Hashed token does not exist in DB");

            StatusCode::BAD_REQUEST
        }
    };

    Ok((return_headers, response_code).into_response())
}
