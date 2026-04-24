use common::serde_utils::disallow_empty_string;
use std::sync::Arc;

use crate::constants::IP_HEADER;
use crate::helpers::{get_ip_addr, validate_cloudflare_token};
use crate::routes::error_message_erasure::ApiError;
use crate::structs::ServerState;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue};
use axum::{Form, debug_handler};
use axum::{http::StatusCode, response::IntoResponse};
use axum_extra::extract::WithRejection;
use common::user_sessions::ServiceType;
use common::utils::get_current_time;
use openid_connect::providers::{
    get_discord_oidc_provider, get_google_oidc_provider, get_microsoft_oidc_provider,
};
use serde::Deserialize;
use serde_with::serde_as;
use tracing::error;

#[serde_as]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CloudflarePayload {
    #[serde(rename = "cf-turnstile-response")]
    #[serde(deserialize_with = "disallow_empty_string")]
    cf_turnstile_response: String,
}

#[debug_handler]
pub(crate) async fn provider(
    headers: HeaderMap,
    State(state): State<Arc<ServerState>>,
    WithRejection(Path(path), _): WithRejection<Path<ServiceType>, ApiError>,
    WithRejection(Form(cloudflare_payload), _): WithRejection<Form<CloudflarePayload>, ApiError>,
) -> Result<impl IntoResponse, StatusCode> {
    let Some(ip_addr) = get_ip_addr(headers) else {
        error!("Request is missing {IP_HEADER} header");

        return Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response());
    };

    let provider = match path {
        ServiceType::Discord => get_discord_oidc_provider().await,
        ServiceType::Google => get_google_oidc_provider().await,
        ServiceType::Microsoft => get_microsoft_oidc_provider().await,
        _ => return Ok(StatusCode::NOT_IMPLEMENTED.into_response()),
    };

    let Some(cloudflare_response) =
        validate_cloudflare_token(&cloudflare_payload.cf_turnstile_response, &ip_addr).await
    else {
        return Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response());
    };

    if !cloudflare_response.success {
        return Ok(StatusCode::UNAUTHORIZED.into_response());
    }

    let oidc_response = provider.fetch_authorization_url().await.unwrap();

    state
        .db
        .create_verification(
            &oidc_response.csrf,
            get_current_time(),
            Some(ip_addr),
            Some(oidc_response.nonce),
        )
        .await;

    let mut return_headers = HeaderMap::new();
    return_headers.append(
        "Location",
        HeaderValue::from_str(&oidc_response.url).unwrap(),
    );

    Ok((return_headers, StatusCode::SEE_OTHER).into_response())
}
