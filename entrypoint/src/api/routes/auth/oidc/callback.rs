use std::str::FromStr;
use std::sync::Arc;

use crate::constants::IP_HEADER;
use crate::helpers::{create_cookie, get_ip_addr};
use crate::routes::error_message_erasure::ApiError;
use crate::structs::ServerState;

use axum::debug_handler;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderName, HeaderValue};
use axum::response::Response;
use axum::{http::StatusCode, response::IntoResponse};
use axum_extra::extract::WithRejection;
use common::constants::SESSION_TOKEN_LENGTH;
use common::string_utils::generate_random_string;
use common::user_sessions::ServiceType;
use common::utils::{get_current_time, get_frontend_domain};
use openid_connect::providers::{
    get_discord_oidc_provider, get_google_oidc_provider, get_microsoft_oidc_provider,
};
use serde::Deserialize;
use tracing::error;

#[derive(Deserialize, Debug)]
pub(crate) struct Payload {
    state: String,
    error: Option<String>,
    code: Option<String>,
}

// TODO: better error handle the hand back to frontend
// eg. error messages if IP mismatch, if DB entry missing, etc
#[debug_handler]
pub(crate) async fn callback(
    headers: HeaderMap,
    State(state): State<Arc<ServerState>>,
    WithRejection(Path(path), _): WithRejection<Path<ServiceType>, ApiError>,
    WithRejection(Query(query), _): WithRejection<Query<Payload>, ApiError>,
) -> Result<impl IntoResponse, StatusCode> {
    // this should only happen if someone is messing with the API
    if query.error.clone().xor(query.code.clone()).is_none() {
        error!("Request is missing one of error or code params");
        return Ok(StatusCode::BAD_REQUEST.into_response());
    }

    let Some(ip_addr) = get_ip_addr(&headers) else {
        error!("Request is missing {IP_HEADER} header");

        return Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response());
    };

    // again, the error only happens if someone is messing with the API
    let provider = match path {
        ServiceType::Discord => get_discord_oidc_provider().await,
        ServiceType::Google => get_google_oidc_provider().await,
        ServiceType::Microsoft => get_microsoft_oidc_provider().await,
        _ => return Ok(StatusCode::NOT_IMPLEMENTED.into_response()),
    };

    let Some(db_verification) = state.db.get_verification(&query.state).await else {
        error!("Saved verification code does exist");
        return Ok(StatusCode::BAD_REQUEST.into_response());
    };

    state.db.delete_verification(&query.state).await;

    if let Some(error) = query.error {
        if error == "access_denied" {
            return Ok(get_home_redirect());
        } else {
            return Ok(StatusCode::BAD_REQUEST.into_response());
        }
    }

    if let Some(saved_ip) = db_verification.ip_addr
        && saved_ip != ip_addr
    {
        error!("Saved verification does not match incoming IP address");

        return Ok(StatusCode::FORBIDDEN.into_response());
    }

    let Some(saved_nonce) = db_verification.nonce else {
        error!("Saved verification does not contain nonce");

        return Ok(StatusCode::BAD_REQUEST.into_response());
    };

    // code should exist at this point
    let code = query.code.unwrap();

    let identifier = provider.exchange_code(&code, &saved_nonce).await.unwrap();

    let token = generate_random_string(SESSION_TOKEN_LENGTH);

    state
        .db
        .create_session(&identifier, path, &token, get_current_time(), &ip_addr)
        .await;

    let mut return_headers = HeaderMap::new();
    return_headers.append(
        "Set-Cookie",
        HeaderValue::from_str(&create_cookie(&token)).unwrap(),
    );
    return_headers.append(
        "Location",
        HeaderValue::from_str(&format!("{}/dashboard", get_frontend_domain())).unwrap(),
    );

    Ok((return_headers, StatusCode::TEMPORARY_REDIRECT).into_response())
}

fn get_home_redirect() -> Response {
    let return_headers = HeaderMap::from_iter([(
        HeaderName::from_str("Location").unwrap(),
        HeaderValue::from_str(&format!("{}/", get_frontend_domain())).unwrap(),
    )]);

    return (return_headers, StatusCode::TEMPORARY_REDIRECT).into_response();
}
