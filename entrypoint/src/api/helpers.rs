use std::{env, sync::LazyLock};

use axum::{
    http::{HeaderMap, HeaderValue, Request},
    response::{IntoResponse, Response},
};
use common::constants::{CLOUDFLARE_TURNSTILE_SECRET_KEY, TOKEN_COOKIE_TTL_SECS};
use regex::bytes::Regex;
use reqwest::{ClientBuilder, StatusCode};
use serde_json::json;
use tower_governor::{GovernorError, key_extractor::KeyExtractor};
use tracing::{error, warn};

use crate::{
    constants::{IP_HEADER, TOKEN_COOKIE_NAME},
    structs::CloudflareResponse,
};

static EMAIL_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$").expect("Regex to compile"));

const CLOUDFLARE_SITE_VERIFY: &str = "https://challenges.cloudflare.com/turnstile/v0/siteverify";

#[derive(Clone)]
pub(crate) struct GovernorIpExtractor;

impl KeyExtractor for GovernorIpExtractor {
    type Key = String;

    fn extract<T>(&self, req: &Request<T>) -> Result<Self::Key, GovernorError> {
        get_ip_addr(req.headers()).ok_or(GovernorError::UnableToExtractKey)
    }
}

pub(crate) fn get_ip_addr(header_map: &HeaderMap) -> Option<String> {
    if cfg!(debug_assertions) {
        return Some(String::new());
    }

    let Some(ip_addr_header) = header_map.get(IP_HEADER) else {
        warn!("Request is missing {IP_HEADER} header");

        return None;
    };

    let Ok(ip_addr) = ip_addr_header.to_str() else {
        warn!("{IP_HEADER} header is not a string");

        return None;
    };

    Some(ip_addr.to_string())
}

pub(crate) fn get_token(header_map: &HeaderMap) -> Option<String> {
    let cookies =
        header_map
            .get("cookie")
            .and_then(|header_value| match header_value.to_str() {
                Ok(value) => Some(value.to_string()),
                Err(_) => return None,
            })?;

    for cookie in cookies.split(";") {
        if let Some((name, value)) = cookie.trim().split_once("=")
            && name == TOKEN_COOKIE_NAME
        {
            return Some(value.to_string());
        }
    }

    None
}

pub(crate) async fn validate_cloudflare_token(
    token: &str,
    ip_addr: &str,
) -> Option<CloudflareResponse> {
    let Ok(cloudflare_secret) = env::var(CLOUDFLARE_TURNSTILE_SECRET_KEY) else {
        error!("{CLOUDFLARE_TURNSTILE_SECRET_KEY} env var is missing");

        return None;
    };

    let client = ClientBuilder::new()
        .gzip(true)
        .https_only(true)
        .build()
        .unwrap();

    let request = client
        .post(CLOUDFLARE_SITE_VERIFY)
        .json(&json!({
            "secret": cloudflare_secret,
            "response": token,
            "remoteip": ip_addr
        }))
        .build()
        .unwrap();

    let response = client.execute(request).await.unwrap();

    Some(response.json::<CloudflareResponse>().await.unwrap())
}

pub(crate) fn is_email_valid(email: &str) -> bool {
    EMAIL_REGEX.is_match(email.as_bytes())
}

// creating cookie manually since the other cookie lib
// assumes I am using `time`, but I use `chrono`
pub(crate) fn create_cookie(token: &str) -> String {
    format!(
        "{TOKEN_COOKIE_NAME}={token}; Max-Age={}; Path=/",
        TOKEN_COOKIE_TTL_SECS
    )
}

/// Creates a Response containing a single `Location` header
pub(crate) fn create_redirect(url: &str, status_code: StatusCode) -> Response {
    let mut return_headers = HeaderMap::new();
    return_headers.append("Location", HeaderValue::from_str(url).unwrap());

    (return_headers, status_code).into_response()
}
