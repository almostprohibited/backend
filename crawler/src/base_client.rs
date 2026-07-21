use std::{
    env,
    str::FromStr,
    sync::{Arc, LazyLock},
    time::Duration,
};

use common::{constants::PROXY_ADDRESS, get_user_agent};
use reqwest::{ClientBuilder as BaseClientBuilder, Proxy, Url, cookie::Jar};
use reqwest_middleware::{ClientBuilder as RetryableClientBuilder, ClientWithMiddleware};
use tracing::debug;

use crate::retry_middleware::get_retry_middleware;

const PAGE_TIMEOUT_SECONDS: u64 = 60;

const PROXY_DOMAINS: [&str; 8] = [
    "italiansportinggoods.com",
    "ellwoodepps.com",
    "x-reload.com",
    "dantesports.com",
    "londerosports.com",
    "internationalshootingsupplies.com",
    "thegundealer.ca",
    "swampdonkeyoutdoors.ca",
];

static COOKIE_JAR: LazyLock<Arc<Jar>> = LazyLock::new(|| Arc::new(Jar::default()));

pub(crate) fn set_cookie(url: &str, cookie: &str) {
    let cookie_jar = COOKIE_JAR.clone();
    cookie_jar.add_cookie_str(cookie, &Url::from_str(url).unwrap());
}

// TODO: randomize ciphers for TLS fingerprinting
pub(crate) fn create_base_client() -> ClientWithMiddleware {
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

        base_client_builder = base_client_builder.proxy(Proxy::custom(move |url| {
            let Some(checked_url) = url.host_str() else {
                debug!("Failed to parse host as string: {url}");

                return None;
            };

            if PROXY_DOMAINS
                .into_iter()
                .any(|proxied_domain| checked_url.ends_with(proxied_domain))
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

    RetryableClientBuilder::new(base_client)
        .with(get_retry_middleware())
        .build()
}
