use crate::constants::TOKEN_COOKIE_NAME;
use std::sync::Arc;

use crate::constants::IP_HEADER;
use crate::helpers::get_ip_addr;
use crate::routes::error_message_erasure::ApiError;
use crate::structs::ServerState;

use axum::debug_handler;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue};
use axum::{http::StatusCode, response::IntoResponse};
use axum_extra::extract::WithRejection;
use common::constants::TOKEN_COOKIE_TTL_SECS;
use common::string_utils::generate_random_string;
use common::user_sessions::ServiceType;
use common::utils::{get_current_time, get_frontend_domain};
use openid_connect::providers::{get_discord_oidc_provider, get_google_oidc_provider};
use serde::Deserialize;
use tracing::{debug, error};

#[derive(Deserialize, Debug)]
// #[serde(deny_unknown_fields)]
pub(crate) struct Payload {
    code: String,
    state: String,
}

#[debug_handler]
pub(crate) async fn callback(
    headers: HeaderMap,
    State(state): State<Arc<ServerState>>,
    WithRejection(Path(path), _): WithRejection<Path<ServiceType>, ApiError>,
    WithRejection(Query(query), _): WithRejection<Query<Payload>, ApiError>,
) -> Result<impl IntoResponse, StatusCode> {
    debug!("{path:?}");
    debug!("{query:?}");

    let Some(ip_addr) = get_ip_addr(headers) else {
        error!("Request is missing {IP_HEADER} header");

        return Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response());
    };

    let provider = match path {
        ServiceType::Discord => get_discord_oidc_provider().await,
        ServiceType::Google => get_google_oidc_provider().await,
        _ => return Ok(StatusCode::NOT_IMPLEMENTED.into_response()),
    };

    let identifier = provider
        .exchange_code(&query.code, &query.state, &ip_addr, &state.db)
        .await
        .unwrap();

    debug!("{identifier}");

    let token = generate_random_string(32);

    // creating cookie manually since the other cookie lib
    // assumes I am using `time`, but I use `chrono`
    let cookie = format!(
        "{TOKEN_COOKIE_NAME}={token}; Max-Age={}; Path=/",
        TOKEN_COOKIE_TTL_SECS
    );

    state
        .db
        .create_session(&identifier, path, &token, get_current_time(), &ip_addr)
        .await;

    let mut return_headers = HeaderMap::new();
    return_headers.append("Set-Cookie", HeaderValue::from_str(&cookie).unwrap());
    return_headers.append(
        "Location",
        HeaderValue::from_str(&format!("{}/dashboard", get_frontend_domain())).unwrap(),
    );

    Ok((return_headers, StatusCode::TEMPORARY_REDIRECT).into_response())
}
