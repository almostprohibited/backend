use std::{str::FromStr, sync::OnceLock};

use reqwest::{
    StatusCode,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use reqwest_middleware::ClientWithMiddleware;
use tracing::{debug, info};

use crate::{
    base_client::{create_base_client, set_cookie},
    errors::CrawlerError,
    request::Request,
    traits::{CrawlerResponse, HttpMethod},
    user_agent::shuffle_user_agent,
};

static REQWEST_CLIENT: OnceLock<ClientWithMiddleware> = OnceLock::new();

#[derive(Copy, Clone)]
pub struct WebClient {}

impl WebClient {
    pub fn set_cookie(url: &str, cookie: &str) {
        set_cookie(url, cookie);
    }

    pub async fn make_web_request(request: Request) -> Result<CrawlerResponse, CrawlerError> {
        let client = REQWEST_CLIENT.get_or_init(|| create_base_client());

        let mut request_builder = match request.method {
            HttpMethod::GET => client.get(request.url.clone()),
            HttpMethod::POST => client.post(request.url.clone()),
        };

        info!(
            "Sending request to {} (body: {:?}) (json: {:?})",
            request.url, request.body, request.json
        );

        if let Some(user_agent) = shuffle_user_agent(&request.url) {
            request_builder = request_builder.header("User-Agent", user_agent);
        }

        if let Some(json) = request.json {
            request_builder = request_builder.json(&json);
        }

        if let Some(body) = request.body {
            request_builder = request_builder.body(body);
        }

        if let Some(headers) = request.headers {
            let mut header_map = HeaderMap::new();

            for (key, value) in headers.iter() {
                header_map.append(HeaderName::from_str(key)?, HeaderValue::from_str(value)?);
            }

            request_builder = request_builder.headers(header_map);
        }

        let response = request_builder.send().await?;

        debug!("{response:?}");

        let status_code = response.status();

        if status_code.is_client_error() && status_code != StatusCode::NOT_FOUND {
            return Err(CrawlerError::InvalidResponseCodeError(status_code));
        }

        let headers = response.headers().clone();

        let body_bytes = response.bytes().await?.to_vec();
        let body_str = String::from_utf8_lossy(&body_bytes).into_owned();

        Ok(CrawlerResponse {
            body: body_str,
            raw_bytes: body_bytes,
            response_code: status_code,
            headers,
        })
    }
}
