use std::sync::Arc;
use std::time::Duration;

use crate::{ServerState, routes::error_message_erasure::ApiError};

use axum::debug_handler;
use axum::http::HeaderMap;
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use axum_extra::extract::WithRejection;
use common::deserialize_disallow_empty_string::disallow_empty_string;
use common::messages::Message;
use discord::ContactWebhook;
use reqwest::ClientBuilder;
use serde::Deserialize;
use serde_json::json;
use serde_with::NoneAsEmptyString;
use serde_with::serde_as;
use tokio::time::sleep;

const CLOUDFLARE_SITE_VERIFY: &str = "https://challenges.cloudflare.com/turnstile/v0/siteverify";

#[cfg(not(debug_assertions))]
const CLOUDFLARE_SECRET: &str = "0x4AAAAAABhALqY4h1CMCPoEahHVHGRIjPI";
#[cfg(debug_assertions)]
const CLOUDFLARE_SECRET: &str = "1x0000000000000000000000000000000AA";

#[derive(Deserialize, Debug)]
struct CloudflareResponse {
    success: bool,
    // cloudflare returns more data in the response
    // I don't care about the extra data
}

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
    let Some(ip_addr_header) = headers.get("X-Real-IP") else {
        return Ok(StatusCode::BAD_GATEWAY);
    };

    let ip_addr = ip_addr_header.to_str().unwrap_or_default();

    let client = ClientBuilder::new()
        .gzip(true)
        .https_only(true)
        .build()
        .unwrap();

    let request = client
        .post(CLOUDFLARE_SITE_VERIFY)
        .json(&json!({
            "secret": CLOUDFLARE_SECRET,
            "response": json.cf_turnstile_response,
            "remoteip": ip_addr
        }))
        .build()
        .unwrap();

    let response = client.execute(request).await.unwrap();
    let parsed_response = response.json::<CloudflareResponse>().await.unwrap();

    if !parsed_response.success {
        return Ok(StatusCode::UNAUTHORIZED);
    }

    let message = Message::new(json.body, ip_addr.to_string(), json.subject, json.email);

    if let Some(ref email) = message.email {
        // I just copied the regex from the Javscript version, but it doesn't work
        // let regex = Regex::new(r"^[\w-\.]+@([\w-]+\.)+[\w-]{2,4}$").unwrap();

        // whatever, just check if it has an "@" and a "." somewhere
        if !email.contains("@") || !email.contains(".") {
            return Ok(StatusCode::BAD_REQUEST);
        }
    };

    if message.body.is_empty() {
        return Ok(StatusCode::BAD_REQUEST);
    }

    ContactWebhook::relay_message(message.clone()).await;

    state.db.insert_message(message).await;

    sleep(Duration::from_secs(1)).await;

    Ok(StatusCode::OK)
}
