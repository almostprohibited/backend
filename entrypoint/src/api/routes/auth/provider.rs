use std::sync::Arc;

use crate::routes::auth::utils::Provider;
use crate::{ServerState, routes::error_message_erasure::ApiError};

use axum::debug_handler;
use axum::extract::Path;
use axum::http::{HeaderMap, HeaderValue};
use axum::{extract::State, http::StatusCode, response::IntoResponse};
use axum_extra::extract::WithRejection;
use openid_connect::providers::DiscordProvider;
use openid_connect::traits::OidcProvider;
use tracing::debug;

#[debug_handler]
pub(crate) async fn provider(
    State(_state): State<Arc<ServerState>>,
    WithRejection(Path(path), _): WithRejection<Path<Provider>, ApiError>,
) -> Result<impl IntoResponse, StatusCode> {
    debug!("{path:?}");

    let url = match path {
        Provider::Discord => DiscordProvider::fetch_authorization_url().await,
    };

    let mut return_headers = HeaderMap::new();
    return_headers.append("Location", HeaderValue::from_str(&url).unwrap());

    Ok((return_headers, StatusCode::TEMPORARY_REDIRECT))
}
