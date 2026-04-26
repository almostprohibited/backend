use std::sync::Arc;

use crate::constants::IP_HEADER;
use crate::helpers::{get_ip_addr, is_email_valid, validate_cloudflare_token};
use crate::{ServerState, routes::error_message_erasure::ApiError};

use axum::debug_handler;
use axum::http::HeaderMap;
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use axum_extra::extract::WithRejection;
use common::messages::Message;
use common::serde_utils::disallow_empty_string;
use discord::get_contact_webhook;
use serde::Deserialize;
use serde_with::NoneAsEmptyString;
use serde_with::serde_as;
use tracing::error;

#[serde_as]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Payload {
    #[serde(rename = "cf-turnstile-response")]
    #[serde(deserialize_with = "disallow_empty_string")]
    cf_turnstile_response: String,
    #[serde(deserialize_with = "disallow_empty_string")]
    body: String,
    #[serde_as(as = "NoneAsEmptyString")]
    #[serde(default)]
    email: Option<String>,
    #[serde_as(as = "NoneAsEmptyString")]
    #[serde(default)]
    subject: Option<String>,
}

#[debug_handler]
pub(crate) async fn contact_handler(
    headers: HeaderMap,
    State(state): State<Arc<ServerState>>,
    WithRejection(Json(json), _): WithRejection<Json<Payload>, ApiError>,
) -> Result<impl IntoResponse, StatusCode> {
    let Some(ip_addr) = get_ip_addr(&headers) else {
        error!("Request is missing {IP_HEADER} header");

        return Ok(StatusCode::INTERNAL_SERVER_ERROR);
    };

    let Some(cloudflare_response) =
        validate_cloudflare_token(&json.cf_turnstile_response, &ip_addr).await
    else {
        return Ok(StatusCode::INTERNAL_SERVER_ERROR);
    };

    if !cloudflare_response.success {
        return Ok(StatusCode::UNAUTHORIZED);
    }

    let message = Message::new(json.body, ip_addr.to_string(), json.subject, json.email);

    if let Some(ref email) = message.email
        && !is_email_valid(email)
    {
        return Ok(StatusCode::BAD_REQUEST);
    };

    if message.body.is_empty() {
        return Ok(StatusCode::BAD_REQUEST);
    }

    state.db.insert_message(message.clone()).await;

    get_contact_webhook().await.relay_message(message).await;

    Ok(StatusCode::OK)
}
