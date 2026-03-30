use std::{
    num::NonZeroUsize,
    sync::{LazyLock, OnceLock},
};

use common::get_user_agent;
use lru::LruCache;
use openidconnect::{
    Nonce,
    reqwest::{Client, ClientBuilder, redirect::Policy},
};
use tokio::sync::Mutex;

pub(crate) struct OidcClientIdentifier {
    pub(crate) ip_addr: String,
    pub(crate) nonce: Nonce,
}

const MAX_CSRF_LOOKUP_SIZE: usize = 1000;

static CSRF_LOOKUP: LazyLock<Mutex<LruCache<String, OidcClientIdentifier>>> = LazyLock::new(|| {
    Mutex::new(LruCache::new(
        NonZeroUsize::new(MAX_CSRF_LOOKUP_SIZE).unwrap(),
    ))
});

static REQWEST_CLIENT: OnceLock<Client> = OnceLock::new();

pub(crate) fn get_reqwest_client() -> &'static Client {
    REQWEST_CLIENT.get_or_init(|| {
        // TODO: see if there is a way to not use vended client
        // this creates an openidconnect reqwest client
        // not the one I have in the workspace
        //
        // this did not work with native reqwest since
        // oidc needs some sort of async trait
        ClientBuilder::new()
            .redirect(Policy::none())
            .user_agent(get_user_agent())
            .https_only(true)
            .build()
            .expect("Valid client to be built")
    })
}

pub(crate) async fn set_csrf_value(csrf: &str, identifier: OidcClientIdentifier) {
    CSRF_LOOKUP.lock().await.push(csrf.to_string(), identifier);
}

pub(crate) async fn get_nonce_by_csrf(csrf: &str, ip_addr: &str) -> Option<Nonce> {
    let mut lookup = CSRF_LOOKUP.lock().await;

    if let Some((_, identifier)) = lookup.pop_entry(csrf)
        && identifier.ip_addr == ip_addr.to_string()
    {
        return Some(identifier.nonce);
    };

    None
}
