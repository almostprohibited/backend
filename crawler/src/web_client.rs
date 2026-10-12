use common::{
    domain_config::{ClientType, default::DEFAULT_SETTINGS},
    string_utils::get_domain,
};
use tracing::info;

use crate::{
    clients::{base::BaseClient, emulated::EmulatedClient},
    errors::CrawlerError,
    request::Request,
    traits::{Client, CrawlerResponse},
};

#[derive(Copy, Clone)]
pub struct WebClient {}

impl WebClient {
    pub fn set_cookie(url: &str, cookie: &str) {
        Self::get_client(url).set_cookie(url, cookie);
    }

    pub async fn make_web_request(request: Request) -> Result<CrawlerResponse, CrawlerError> {
        info!(
            "Sending request to {} (client: {:?}) (body: {:?}) (json: {:?}) (proxy: {:?}) (sigs: {})",
            request.url,
            request.get_client_settings().client_type,
            request.body,
            request.json,
            request.get_client_settings().proxy_type,
            request.get_client_settings().include_http_sigs,
        );

        Self::get_client(&request.url).send_request(request).await
    }

    fn get_client(url: &str) -> &'static dyn Client {
        let domain = get_domain(url);

        let Some(settings) = DEFAULT_SETTINGS.get(&domain) else {
            return &BaseClient {};
        };

        match settings.client_type {
            ClientType::Emulated => &EmulatedClient {},
            _ => &BaseClient {},
        }
    }
}
