use async_trait::async_trait;

#[async_trait]
pub trait OidcProvider {
    async fn fetch_authorization_url() -> String;
    async fn exchange_code(code: String, state: String) -> String;
}
