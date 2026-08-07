use common::{http_sig::create_request_headers, string_utils::get_domain};
use reqwest::header::{HeaderMap, HeaderValue};
use tracing::{debug, info};

use crate::{
    clients::{
        base::{send_request as send_base_request, set_cookie as set_base_cookie},
        emulated::{send_request as send_emulated_request, set_cookie as set_emulated_cookie},
    },
    constants::{EMULATED_DOMAINS, HTTP_SIG_EXCLUDED_DOMAINS},
    errors::CrawlerError,
    request::Request,
    traits::CrawlerResponse,
};

#[derive(Copy, Clone)]
pub struct WebClient {}

impl WebClient {
    pub fn set_cookie(url: &str, cookie: &str) {
        match Self::should_use_emulated_client(url) {
            true => set_emulated_cookie(url, cookie),
            false => set_base_cookie(url, cookie),
        }
    }

    pub async fn make_web_request(request: Request) -> Result<CrawlerResponse, CrawlerError> {
        let should_emulate = Self::should_use_emulated_client(&request.url);

        info!(
            "Sending request to {} (emulated: {should_emulate}) (body: {:?}) (json: {:?})",
            request.url, request.body, request.json
        );

        match should_emulate {
            true => send_emulated_request(request).await,
            false => {
                let sig_headers = match HTTP_SIG_EXCLUDED_DOMAINS
                    .iter()
                    .any(|excluded_domain| request.url.contains(excluded_domain))
                {
                    true => None,
                    false => Some(Self::get_http_sig_headers(&request)),
                };

                send_base_request(request, sig_headers).await
            }
        }
    }

    fn should_use_emulated_client(request_url: &str) -> bool {
        EMULATED_DOMAINS.contains(&get_domain(request_url).as_str())
    }

    fn get_http_sig_headers(request: &Request) -> HeaderMap {
        let sig_headers = create_request_headers(&request.url);

        debug!(
            "HTTP signatures\n{}\n{}\n{:?}",
            sig_headers.signature, sig_headers.signature_input, sig_headers.signature_agent
        );

        let mut return_headers = HeaderMap::new();

        return_headers.insert(
            "Signature",
            HeaderValue::from_str(&sig_headers.signature).unwrap(),
        );

        return_headers.insert(
            "Signature-Input",
            HeaderValue::from_str(&sig_headers.signature_input).unwrap(),
        );

        if let Some(sig_agent) = sig_headers.signature_agent {
            return_headers.insert(
                "Signature-Agent",
                HeaderValue::from_str(&sig_agent).unwrap(),
            );
        }

        return_headers
    }
}
