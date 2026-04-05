use std::sync::Arc;

use crate::constants::IP_HEADER;
use crate::helpers::get_ip_addr;
use crate::routes::error_message_erasure::ApiError;
use crate::structs::ServerState;

use axum::debug_handler;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue};
use axum::{http::StatusCode, response::IntoResponse};
use axum_extra::extract::WithRejection;
use common::user_sessions::ServiceType;
use openid_connect::providers::{get_discord_oidc_provider, get_google_oidc_provider};
use tracing::{debug, error};

#[debug_handler]
pub(crate) async fn provider(
    headers: HeaderMap,
    State(state): State<Arc<ServerState>>,
    WithRejection(Path(path), _): WithRejection<Path<ServiceType>, ApiError>,
) -> Result<impl IntoResponse, StatusCode> {
    debug!("{path:?}");

    let Some(ip_addr) = get_ip_addr(headers) else {
        error!("Request is missing {IP_HEADER} header");

        return Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response());
    };

    let provider = match path {
        ServiceType::Discord => get_discord_oidc_provider().await,
        ServiceType::Google => get_google_oidc_provider().await,
        _ => return Ok(StatusCode::NOT_IMPLEMENTED.into_response()),
    };

    let mut return_headers = HeaderMap::new();
    return_headers.append(
        "Location",
        HeaderValue::from_str(
            &provider
                .fetch_authorization_url(&ip_addr, &state.db)
                .await
                .unwrap(),
        )
        .unwrap(),
    );

    Ok((return_headers, StatusCode::TEMPORARY_REDIRECT).into_response())
}
