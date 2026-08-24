use std::{
    env,
    hash::{DefaultHasher, Hash, Hasher},
    str::FromStr,
    sync::{Arc, LazyLock, OnceLock},
    time::Duration,
};

use chrono::{Datelike, Utc};
use common::constants::PROXY_ADDRESS;
use tracing::debug;
use wreq::{
    Client, Proxy, StatusCode,
    cookie::Jar,
    header::{HeaderMap, HeaderName, HeaderValue},
    retry::Policy,
};
use wreq_util::{Emulation, Platform, Profile};

use crate::{
    constants::PAGE_TIMEOUT_SECONDS,
    errors::CrawlerError,
    request::Request,
    traits::{CrawlerResponse, HttpMethod},
};

static EMULATED_CLIENT: OnceLock<Client> = OnceLock::new();
static EMULATED_COOKIE_JAR: LazyLock<Arc<Jar>> = LazyLock::new(|| Arc::new(Jar::default()));

const PROFILES: [Profile; 4] = [
    // Profile::Chrome149,
    // Profile::Chrome148,
    // Profile::Chrome147,
    // Profile::Chrome146,
    // Profile::Edge148,
    // Profile::Edge147,
    // Profile::Edge146,
    // Profile::Edge145,
    Profile::Firefox151,
    Profile::Firefox150,
    Profile::Firefox149,
    Profile::Firefox148,
];

fn hash_host_to_index(max_items: u64) -> u64 {
    let datetime = Utc::now();

    let mut hasher = DefaultHasher::new();

    datetime.day().hash(&mut hasher);
    datetime.month().hash(&mut hasher);
    datetime.year().hash(&mut hasher);

    hasher.finish() % max_items
}

fn shuffle_profile() -> Emulation {
    let emulation_profile = Emulation::builder()
        .profile(PROFILES[hash_host_to_index(PROFILES.len() as u64) as usize])
        // TODO: consider randomizing OS
        .platform(Platform::Windows)
        .http2(true)
        .headers(true);

    emulation_profile.build()
}

pub(crate) fn set_cookie(url: &str, cookie: &str) {
    let cookie_jar = EMULATED_COOKIE_JAR.clone();
    cookie_jar.add(cookie, url);
}

fn create_emulated_client() -> Client {
    let mut emulated_client_builder = Client::builder()
        .emulation(shuffle_profile())
        .gzip(true)
        .connect_timeout(Duration::from_secs(PAGE_TIMEOUT_SECONDS))
        .https_only(true)
        .redirect(Default::default())
        .cookie_provider(EMULATED_COOKIE_JAR.clone())
        .retry(Policy::default())
        .connection_verbose(true);

    if let Ok(proxy_address) = env::var(PROXY_ADDRESS) {
        debug!("Configuring proxy");

        emulated_client_builder = emulated_client_builder.proxy(Proxy::all(proxy_address).unwrap());
    }

    emulated_client_builder
        .build()
        .expect("Valid base reqwest to be built")
}

pub(crate) async fn send_request(request: Request) -> Result<CrawlerResponse, CrawlerError> {
    let client = EMULATED_CLIENT.get_or_init(|| create_emulated_client());

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
            header_map.append(
                HeaderName::from_str(key).unwrap(),
                HeaderValue::from_str(value).unwrap(),
            );
        }

        request_builder = request_builder.headers(header_map);
    }

    let response = request_builder.send().await.unwrap();

    debug!("{response:?}");

    let status_code = response.status();

    if status_code.is_client_error() && status_code != StatusCode::NOT_FOUND {
        return Err(CrawlerError::InvalidResponseCodeError(status_code));
    }

    let headers = response.headers().clone();

    let body_bytes = response.bytes().await.unwrap().to_vec();
    let body_str = String::from_utf8_lossy(&body_bytes).into_owned();

    Ok(CrawlerResponse {
        body: body_str,
        raw_bytes: body_bytes,
        response_code: status_code,
        headers,
    })
}
