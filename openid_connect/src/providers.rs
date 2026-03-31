use std::env;

use common::{
    constants::{
        OIDC_CLIENT_ID_DISCORD, OIDC_CLIENT_ID_GOOGLE, OIDC_CLIENT_SECRET_DISCORD,
        OIDC_CLIENT_SECRET_GOOGLE,
    },
    utils::get_backend_domain,
};
use openidconnect::core::CoreAuthPrompt;

use crate::base_provider::{BaseOidcProvider, BaseOidcProviderBuilder};

pub async fn get_discord_oidc_provider() -> BaseOidcProvider {
    let client_id = env::var(OIDC_CLIENT_ID_DISCORD).unwrap().to_string();
    let client_secret = env::var(OIDC_CLIENT_SECRET_DISCORD).unwrap().to_string();

    BaseOidcProviderBuilder::new(
        "https://discord.com",
        &format!("{}/api/auth/discord/callback", get_backend_domain()),
        &client_id,
        &client_secret,
    )
    .with_authorization_url("https://discord.com/oauth2/authorize")
    .build()
}

pub async fn get_google_oidc_provider() -> BaseOidcProvider {
    let client_id = env::var(OIDC_CLIENT_ID_GOOGLE).unwrap().to_string();
    let client_secret = env::var(OIDC_CLIENT_SECRET_GOOGLE).unwrap().to_string();

    BaseOidcProviderBuilder::new(
        "https://accounts.google.com",
        &format!("{}/api/auth/google/callback", get_backend_domain()),
        &client_id,
        &client_secret,
    )
    .with_prompt(CoreAuthPrompt::Consent)
    .build()
}
