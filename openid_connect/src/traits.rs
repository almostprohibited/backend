use async_trait::async_trait;

use crate::errors::OidcError;

#[async_trait]
pub trait OidcProvider {
    async fn fetch_authorization_url(ip_addr: &str) -> Result<String, OidcError>;
    async fn exchange_code(code: &str, state: &str, ip_addr: &str) -> Result<String, OidcError>;
}

pub struct OidcAuthorizationProps {
    pub url: String,
    pub csrf: String,
    pub nonce: String,
}
