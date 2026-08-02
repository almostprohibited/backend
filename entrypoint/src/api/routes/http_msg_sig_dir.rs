use std::str::FromStr;

use axum::{
    Json,
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::IntoResponse,
};
use common::http_sig::{JWK_KEYS, create_directory_headers};
use tracing::debug;

pub(crate) async fn http_sigs(headers: HeaderMap) -> Result<impl IntoResponse, StatusCode> {
    let Some(host) = headers.get("host") else {
        debug!("Request missing host header");

        return Ok(StatusCode::BAD_REQUEST.into_response());
    };

    let host_val = host.to_str().unwrap();

    debug!("{host_val}");

    let http_sigs = create_directory_headers(host_val);

    let response = Json::from(JWK_KEYS.clone());

    let return_headers: HeaderMap = vec![
        (
            "Content-Type",
            "application/http-message-signatures-directory+json",
        ),
        ("Signature", &http_sigs.signature),
        ("Signature-Input", &http_sigs.signature_input),
    ]
    .into_iter()
    .map(|(name, value)| {
        (
            HeaderName::from_str(name).unwrap(),
            HeaderValue::from_str(value).unwrap(),
        )
    })
    .collect();

    Ok((return_headers, response).into_response())
}
