use std::{str::FromStr, time::Duration};

use reqwest::{
    ClientBuilder as BaseClientBuilder,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use reqwest_middleware::ClientBuilder as RetryableClientBuilder;
use reqwest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};
use tracing::{debug, info};

use crate::{
    errors::CrawlerError,
    request::Request,
    traits::{Crawler, HttpMethod},
};

const PAGE_TIMEOUT_SECONDS: u64 = 30;
const USER_AGENT: &str =
    "almostprohibited/1.0 (+https://almostprohibited.ca/contact/; hello@almostprohibited.ca)";

#[derive(Copy, Clone)]
pub struct UnprotectedCrawler {}

impl UnprotectedCrawler {
    pub fn new() -> Self {
        Self {}
    }
}

impl Crawler for UnprotectedCrawler {
    async fn make_web_request(&self, request: Request) -> Result<String, CrawlerError> {
        let base_client = BaseClientBuilder::new()
            .gzip(true)
            .http1_ignore_invalid_headers_in_responses(true)
            .timeout(Duration::from_secs(PAGE_TIMEOUT_SECONDS))
            .user_agent(USER_AGENT)
            .https_only(true)
            .connection_verbose(true)
            .build()?;

        let retry_strat = ExponentialBackoff::builder()
            .retry_bounds(Duration::from_secs(1), Duration::from_secs(60))
            .build_with_max_retries(2);
        let retry_middleware = RetryTransientMiddleware::new_with_policy(retry_strat);

        let client = RetryableClientBuilder::new(base_client)
            .with(retry_middleware)
            .build();

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

        let sent_request = request_builder.send().await?;

        debug!("{:?}", sent_request);

        Ok(sent_request.text().await?)
    }
}
