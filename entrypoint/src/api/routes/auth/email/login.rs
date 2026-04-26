use common::serde_utils::disallow_empty_string;
use common::string_utils::{generate_random_code, sha256_hash_string};
use common::utils::get_current_time;
use email::send_otp_email;
use std::sync::Arc;

use crate::constants::IP_HEADER;
use crate::helpers::{get_ip_addr, is_email_valid, validate_cloudflare_token};
use crate::routes::error_message_erasure::ApiError;
use crate::structs::ServerState;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::{Json, debug_handler};
use axum::{http::StatusCode, response::IntoResponse};
use axum_extra::extract::WithRejection;
use serde::Deserialize;
use serde_with::serde_as;
use tracing::error;

#[serde_as]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Payload {
    #[serde(rename = "cf-turnstile-response")]
    #[serde(deserialize_with = "disallow_empty_string")]
    cf_turnstile_response: String,
    email: String,
}

#[debug_handler]
pub(crate) async fn email_login(
    headers: HeaderMap,
    State(state): State<Arc<ServerState>>,
    WithRejection(Json(payload), _): WithRejection<Json<Payload>, ApiError>,
) -> Result<impl IntoResponse, StatusCode> {
    let Some(ip_addr) = get_ip_addr(&headers) else {
        error!("Request is missing {IP_HEADER} header");

        return Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response());
    };

    if !is_email_valid(&payload.email) {
        error!("Request is missing valid email");

        return Ok(StatusCode::BAD_REQUEST.into_response());
    }

    let Some(cloudflare_response) =
        validate_cloudflare_token(&payload.cf_turnstile_response, &ip_addr).await
    else {
        error!("Failed to call Cloudflare");

        return Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response());
    };

    if !cloudflare_response.success {
        error!("Request failed Cloudflare check");

        return Ok(StatusCode::UNAUTHORIZED.into_response());
    }

    let otp_code = generate_random_code();

    // technically I don't need to append the code, but it'll make
    // it "random" instead of being the same hash in the DB
    let nonce = format!("{}{otp_code}", payload.email);

    state
        .db
        .create_verification(
            &otp_code,
            get_current_time(),
            Some(ip_addr),
            Some(sha256_hash_string(&nonce)),
        )
        .await;

    if send_otp_email(&payload.email, &otp_code).await {
        return Ok(StatusCode::OK.into_response());
    } else {
        return Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response());
    }
}
