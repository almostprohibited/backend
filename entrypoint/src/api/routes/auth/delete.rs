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
pub(crate) async fn delete_handler(
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
        HeaderValue::from_str(&format!("{TOKEN_COOKIE_NAME}=; Max-Age-0; Path=/")).unwrap(),
    );

    let response_code = match state.db.nuke_account(&token).await {
        true => StatusCode::OK,
        false => {
            // might not be client error at this stage since I could have
            // messed up the account information
            debug!("Failed to delete account");

            StatusCode::BAD_REQUEST
        }
    };

    Ok((return_headers, response_code).into_response())
}
