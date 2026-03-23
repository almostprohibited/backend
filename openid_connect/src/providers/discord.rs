use std::env;

use async_trait::async_trait;
use common::constants::OIDC_CLIENT_ID_DISCORD;
use openidconnect::{
    AuthUrl, AuthenticationFlow, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce, RedirectUrl,
    core::{CoreClient, CoreProviderMetadata, CoreResponseType},
    reqwest::{ClientBuilder, redirect::Policy},
};
use tracing::debug;

use crate::traits::OidcProvider;

pub struct DiscordProvider {}

#[async_trait]
impl OidcProvider for DiscordProvider {
    async fn fetch_authorization_url() -> String {
        // TODO: see if there is a way to not use vended client
        // this creates an openidconnect reqwest client
        // not the one I have in the workspace
        //
        // this did not work with native reqwest since
        // oidc needs some sort of async trait
        let http_client = ClientBuilder::new()
            .redirect(Policy::none())
            .build()
            .expect("Valid client to be built");

        let client_id = ClientId::new(env::var(OIDC_CLIENT_ID_DISCORD).unwrap().to_string());
        let client_secret =
            ClientSecret::new(env::var(OIDC_CLIENT_ID_DISCORD).unwrap().to_string());

        let redirect_url =
            RedirectUrl::new("http://localhost:3001/api/auth/discord/callback".to_string())
                .unwrap();

        let issuer_url = IssuerUrl::new("https://discord.com".to_string()).unwrap();

        let provider_metadata = CoreProviderMetadata::discover_async(issuer_url, &http_client)
            .await
            .unwrap()
            .set_authorization_endpoint(
                AuthUrl::new("https://discord.com/oauth2/authorize".to_string()).unwrap(),
            );

        let oidc_client =
            CoreClient::from_provider_metadata(provider_metadata, client_id, Some(client_secret))
                .set_redirect_uri(redirect_url);

        let (auth_url, csrf, nonce) = oidc_client
            .authorize_url(
                AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .url();

        debug!("{auth_url}");
        debug!("{:?}", csrf.secret());
        debug!("{:?}", nonce.secret());

        auth_url.to_string()
    }

    async fn exchange_code(code: String, state: String) -> String {
        "".to_string()
    }
}
