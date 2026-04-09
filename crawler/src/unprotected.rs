use std::{env, str::FromStr, sync::OnceLock, time::Duration};

use common::{constants::PROXY_ADDRESS, get_user_agent};
use reqwest::{
    ClientBuilder as BaseClientBuilder, Proxy, Url,
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

const PROXY_DOMAINS: [&str; 5] = [
    "www.italiansportinggoods.com",
    "ellwoodepps.com",
    "x-reload.com",
    "www.dantesports.com",
    "www.londerosports.com",
];

static REQWEST_CLIENT: OnceLock<ClientWithMiddleware> = OnceLock::new();

#[derive(Copy, Clone)]
pub struct UnprotectedCrawler {}

impl UnprotectedCrawler {
    fn create_client() -> &'static ClientWithMiddleware {
        REQWEST_CLIENT.get_or_init(|| {
            let mut base_client_builder = BaseClientBuilder::new()
                .gzip(true)
                .http1_ignore_invalid_headers_in_responses(true)
                .timeout(Duration::from_secs(PAGE_TIMEOUT_SECONDS))
                .user_agent(get_user_agent())
                .https_only(true)
                .cookie_store(true)
                .connection_verbose(true);

            if let Ok(proxy_address) = env::var(PROXY_ADDRESS) {
                debug!("Configuring proxy");

                let proxy_url = Url::parse(&proxy_address).expect("Valid proxy domain");

                base_client_builder = base_client_builder.proxy(Proxy::custom(move |url| {
                    let Some(checked_url) = url.host_str() else {
                        debug!("Failed to parse host as string: {url}");

                        return None;
                    };

                    if PROXY_DOMAINS
                        .into_iter()
                        .any(|proxied_domain| checked_url == proxied_domain)
                    {
                        debug!("Proxying {checked_url}");

                        return Some(proxy_url.clone());
                    }

                    None
                }));
            }

            let base_client = base_client_builder
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

    pub async fn make_web_request(request: Request) -> Result<CrawlerResponse, CrawlerError> {
        let client = Self::create_client();

        let mut request_builder = match request.method {
            HttpMethod::GET => client.get(request.url.clone()),
            HttpMethod::POST => client.post(request.url.clone()),
        };

        info!(
            "Sending request to {} (body: {:?}) (json: {:?})",
            request.url, request.body, request.json
        );

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

        let headers = response.headers().clone();

        let body_bytes = response.bytes().await?.to_vec();
        let body_str = String::from_utf8_lossy(&body_bytes).into_owned();

        Ok(CrawlerResponse {
            body: body_str,
            raw_bytes: body_bytes,
            headers,
        })
    }
}
