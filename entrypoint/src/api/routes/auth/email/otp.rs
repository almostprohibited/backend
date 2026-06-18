use common::constants::SESSION_TOKEN_LENGTH;
use common::db::ServiceType;
use common::serde_utils::disallow_empty_string;
use common::string_utils::{generate_random_string, sha256_hash_string};
use std::sync::Arc;

use crate::constants::IP_HEADER;
use crate::helpers::{create_cookie, get_ip_addr, is_email_valid, validate_cloudflare_token};
use crate::routes::error_message_erasure::ApiError;
use crate::structs::ServerState;

use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue};
use axum::{Json, debug_handler};
use axum::{http::StatusCode, response::IntoResponse};
use axum_extra::extract::WithRejection;
use common::utils::get_current_time;
use serde::Deserialize;
use serde_with::serde_as;
use tracing::error;

#[serde_as]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Payload {
    #[serde(alias = "cf-turnstile-response")]
    #[serde(deserialize_with = "disallow_empty_string")]
    cf_turnstile_response: String,
    email: String,
    #[serde(alias = "otp-code")]
    otp_code: String,
}

#[debug_handler]
pub(crate) async fn email_otp(
    headers: HeaderMap,
    State(state): State<Arc<ServerState>>,
    WithRejection(Json(payload), _): WithRejection<Json<Payload>, ApiError>,
) -> Result<impl IntoResponse, StatusCode> {
    let Some(ip_addr) = get_ip_addr(&headers) else {
        error!("Request is missing {IP_HEADER} header");

        return Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response());
    };

    if !is_email_valid(&payload.email) {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    }

    let Some(cloudflare_response) =
        validate_cloudflare_token(&payload.cf_turnstile_response, &ip_addr).await
    else {
        return Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response());
    };

    if !cloudflare_response.success {
        return Ok(StatusCode::UNAUTHORIZED.into_response());
    }

    let result = state.db.get_verification(&payload.otp_code).await;

    let Some(verification) = result else {
        error!("Saved verification code does exist");
        return Ok(StatusCode::BAD_REQUEST.into_response());
    };

    state.db.delete_verification(&payload.otp_code).await;

    let nonce = format!("{}{}", payload.email, payload.otp_code);

    if let Some(hashed_email) = verification.nonce
        && hashed_email == sha256_hash_string(&nonce)
    {
        let token = generate_random_string(SESSION_TOKEN_LENGTH);

        state
            .db
            .create_session(
                &payload.email,
                ServiceType::Email,
                &token,
                get_current_time(),
                &ip_addr,
            )
            .await;

        let mut return_headers = HeaderMap::new();
        return_headers.append(
            "Set-Cookie",
            HeaderValue::from_str(&create_cookie(&token)).unwrap(),
        );

        return Ok((return_headers, StatusCode::OK).into_response());
    } else {
        error!("Saved verification does not match incoming email");

        return Ok(StatusCode::BAD_REQUEST.into_response());
    }
}
