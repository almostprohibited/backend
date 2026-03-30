use axum::http::HeaderMap;
use tracing::warn;

use crate::constants::IP_HEADER;

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
