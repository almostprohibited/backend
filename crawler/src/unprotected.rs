use std::{collections::HashMap, str::FromStr, sync::OnceLock, time::Duration};

use reqwest::{
    ClientBuilder as BaseClientBuilder,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use reqwest_middleware::{ClientBuilder as RetryableClientBuilder, ClientWithMiddleware};
use reqwest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};
use tracing::{debug, info};

use crate::{
    errors::CrawlerError,
    request::Request,
    traits::{CrawlerResponse, HttpMethod},
};

const PAGE_TIMEOUT_SECONDS: u64 = 30;
const PAGE_MIN_SECS_BACKOFF: u64 = 60;
const PAGE_MAX_SECS_BACKOFF: u64 = 120;
const MAX_RETRY: u32 = 3;

const USER_AGENT: &str =
    "almostprohibited/1.0 (+https://almostprohibited.ca/contact/; hello@almostprohibited.ca)";

static REQWEST_CLIENT: OnceLock<ClientWithMiddleware> = OnceLock::new();

#[derive(Copy, Clone)]
pub struct UnprotectedCrawler {}

impl Default for UnprotectedCrawler {
    fn default() -> Self {
        Self::new()
    }
}

impl UnprotectedCrawler {
    pub fn new() -> Self {
        Self {}
    }

    fn create_client() -> &'static ClientWithMiddleware {
        REQWEST_CLIENT.get_or_init(|| {
            let base_client = BaseClientBuilder::new()
                .gzip(true)
                .http1_ignore_invalid_headers_in_responses(true)
                .timeout(Duration::from_secs(PAGE_TIMEOUT_SECONDS))
                .user_agent(USER_AGENT)
                .https_only(true)
                .build()
                .expect("Valid base reqwest to be built");

            let retry_strat = ExponentialBackoff::builder()
                .retry_bounds(
                    Duration::from_secs(PAGE_MIN_SECS_BACKOFF),
                    Duration::from_secs(PAGE_MAX_SECS_BACKOFF),
                )
                .build_with_max_retries(MAX_RETRY);
            let retry_middleware = RetryTransientMiddleware::new_with_policy(retry_strat);

            RetryableClientBuilder::new(base_client)
                .with(retry_middleware)
                .build()
        })
    }

    pub async fn make_web_request(
        &self,
        request: Request,
    ) -> Result<CrawlerResponse, CrawlerError> {
        let client = Self::create_client();

        let mut request_builder = match request.method {
            HttpMethod::GET => client.get(request.url.clone()),
            HttpMethod::POST => client.post(request.url.clone()),
        };

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

        info!("Sending request to {}", request.url);

        let response = request_builder.send().await?;

        debug!("{response:?}");

        let headers = response.headers().clone();

        let mut cookies = HashMap::new();
        for cookie in response.cookies() {
            cookies.insert(cookie.name().into(), cookie.value().into());
        }

        let body = response.text().await?;

        Ok(CrawlerResponse {
            body,
            headers,
            cookies,
        })
    }
}
