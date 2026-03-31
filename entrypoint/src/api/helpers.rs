use axum::http::HeaderMap;
use base64::{Engine, prelude::BASE64_STANDARD};
use sha2::{Digest, Sha256};
use tracing::warn;

use crate::constants::{IP_HEADER, TOKEN_COOKIE_NAME};

pub(crate) fn get_ip_addr(header_map: HeaderMap) -> Option<String> {
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

pub(crate) fn hash_string(token: &str) -> String {
    let hash = Sha256::digest(token.as_bytes());
    BASE64_STANDARD.encode(&hash)
}
