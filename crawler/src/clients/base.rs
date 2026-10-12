use std::{
    env,
    str::FromStr,
    sync::{Arc, LazyLock, OnceLock},
    time::Duration,
};

use async_trait::async_trait;
use common::{
    constants::PROXY_ADDRESS,
    domain_config::{ProxyType, default::DEFAULT_SETTINGS},
    get_user_agent,
    string_utils::get_domain,
};
use reqwest::{ClientBuilder as BaseClientBuilder, Proxy as BaseProxy, Url, cookie::Jar};
use reqwest::{
    StatusCode,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use reqwest_middleware::{ClientBuilder as RetryableClientBuilder, ClientWithMiddleware};
use tracing::debug;

use crate::{
    constants::PAGE_TIMEOUT_SECONDS,
    errors::CrawlerError,
    request::Request,
    retry_middleware::get_retry_middleware,
    traits::{Client, CrawlerResponse, HttpMethod},
    utils::get_http_sig_headers,
};

static REQWEST_CLIENT: OnceLock<ClientWithMiddleware> = OnceLock::new();
static COOKIE_JAR: LazyLock<Arc<Jar>> = LazyLock::new(|| Arc::new(Jar::default()));

pub(crate) struct BaseClient;

#[async_trait]
impl Client for BaseClient {
    fn set_cookie(&self, url: &str, cookie: &str) {
        let cookie_jar = COOKIE_JAR.clone();
        cookie_jar.add_cookie_str(cookie, &Url::from_str(url).unwrap());
    }

    async fn send_request(&self, request: Request) -> Result<CrawlerResponse, CrawlerError> {
        let client = REQWEST_CLIENT.get_or_init(|| create_base_client());

        let mut request_builder = match request.method {
            HttpMethod::GET => client.get(request.url.clone()),
            HttpMethod::POST => client.post(request.url.clone()),
        };

        if let Some(json) = &request.json {
            request_builder = request_builder.json(&json);
        }

        if let Some(body) = &request.body {
            request_builder = request_builder.body(body.clone());
        }

        if let Some(headers) = &request.headers {
            let mut header_map = HeaderMap::new();

            for (key, value) in headers.iter() {
                header_map.append(HeaderName::from_str(key)?, HeaderValue::from_str(value)?);
            }

            request_builder = request_builder.headers(header_map);
        }

        if request.get_client_settings().include_http_sigs {
            let mut reqwest_headers = HeaderMap::new();

            for (key, value) in get_http_sig_headers(&request).iter() {
                reqwest_headers.append(HeaderName::from_str(key)?, HeaderValue::from_str(value)?);
            }

            request_builder = request_builder.headers(reqwest_headers);
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

fn create_base_client() -> ClientWithMiddleware {
    let mut base_client_builder = BaseClientBuilder::new()
        .gzip(true)
        .http1_ignore_invalid_headers_in_responses(true)
        .timeout(Duration::from_secs(PAGE_TIMEOUT_SECONDS))
        .user_agent(get_user_agent())
        .https_only(true)
        .cookie_provider(COOKIE_JAR.clone())
        .connection_verbose(true);

    if let Ok(proxy_address) = env::var(PROXY_ADDRESS) {
        debug!("Configuring proxy");

        let proxy_url = Url::parse(&proxy_address).expect("Valid proxy domain");

        base_client_builder = base_client_builder.proxy(BaseProxy::custom(move |url| {
            let checked_url = get_domain(&url.to_string());

            if let Some(settings) = DEFAULT_SETTINGS.get(&checked_url)
                && matches!(settings.proxy_type, ProxyType::Normal)
            {
                debug!("Proxying {checked_url}");

                return Some(proxy_url.clone());
            }

            debug!("Skip proxying {checked_url}");

            None
        }));
    }

    let base_client = base_client_builder
        .build()
        .expect("Valid base reqwest to be built");

    RetryableClientBuilder::new(base_client)
        .with(get_retry_middleware())
        .build()
}
