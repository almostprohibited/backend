use std::{collections::HashMap, str::FromStr, time::Duration};

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

#[derive(Copy, Clone)]
pub struct UnprotectedCrawler {
    min_backoff: u64,
    max_backoff: u64,
    max_retry: u32,
}

impl UnprotectedCrawler {
    pub fn new() -> Self {
        Self {
            min_backoff: PAGE_MIN_SECS_BACKOFF,
            max_backoff: PAGE_MAX_SECS_BACKOFF,
            max_retry: MAX_RETRY,
        }
    }

    pub fn with_min_secs_backoff(mut self, backoff: u64) -> Self {
        self.min_backoff = backoff;
        self
    }

    pub fn with_max_secs_backoff(mut self, backoff: u64) -> Self {
        self.max_backoff = backoff;
        self
    }

    pub fn with_max_retry(mut self, retries: u32) -> Self {
        self.max_retry = retries;
        self
    }

    fn create_client(&self) -> Result<ClientWithMiddleware, CrawlerError> {
        let base_client = BaseClientBuilder::new()
            .gzip(true)
            .http1_ignore_invalid_headers_in_responses(true)
            .timeout(Duration::from_secs(PAGE_TIMEOUT_SECONDS))
            .user_agent(USER_AGENT)
            .https_only(true)
            .connection_verbose(true)
            .build()?;

        let retry_strat = ExponentialBackoff::builder()
            .retry_bounds(
                Duration::from_secs(self.min_backoff),
                Duration::from_secs(self.max_backoff),
            )
            .build_with_max_retries(self.max_retry);
        let retry_middleware = RetryTransientMiddleware::new_with_policy(retry_strat);

        Ok(RetryableClientBuilder::new(base_client)
            .with(retry_middleware)
            .build())
    }

    pub async fn make_web_request(
        &self,
        request: Request,
    ) -> Result<CrawlerResponse, CrawlerError> {
        let client = self.create_client()?;

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
