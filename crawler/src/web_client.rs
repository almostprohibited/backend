use common::{
    domain_config::{ClientSettings, ClientType, default::DEFAULT_SETTINGS},
    string_utils::get_domain,
};
use tracing::info;

use crate::{
    clients::{base::BaseClient, browser::BrowserClient, emulated::EmulatedClient},
    errors::CrawlerError,
    request::Request,
    traits::{Client, CrawlerResponse},
};

#[derive(Copy, Clone)]
pub struct WebClient {}

impl WebClient {
    // TODO: refactor this, it won't respect client settings
    // correctly (eg. caller cannot define client used)
    pub async fn set_cookie(url: &str, cookie: &str) {
        let domain = get_domain(url);

        let Some(settings) = DEFAULT_SETTINGS.get(&domain) else {
            BaseClient {}.set_cookie(url, cookie).await;
            return;
        };

        Self::get_client(settings).set_cookie(url, cookie).await;
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

        Self::get_client(&request.get_client_settings())
            .send_request(request)
            .await
    }

    fn get_client(settings: &ClientSettings) -> &'static dyn Client {
        match settings.client_type {
            ClientType::Emulated => &EmulatedClient {},
            ClientType::Browser => &BrowserClient {},
            _ => &BaseClient {},
        }
    }

    pub async fn kill_clients() {
        BrowserClient::kill_browser().await;
    }
}
