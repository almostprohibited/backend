use std::env;

use common::{
    constants::{
        OIDC_CLIENT_ID_DISCORD, OIDC_CLIENT_ID_GOOGLE, OIDC_CLIENT_ID_MICROSOFT,
        OIDC_CLIENT_SECRET_DISCORD, OIDC_CLIENT_SECRET_GOOGLE, OIDC_CLIENT_SECRET_MICROSOFT,
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

pub async fn get_microsoft_oidc_provider() -> BaseOidcProvider {
    let client_id = env::var(OIDC_CLIENT_ID_MICROSOFT).unwrap().to_string();
    let client_secret = env::var(OIDC_CLIENT_SECRET_MICROSOFT).unwrap().to_string();

    // of course microsoft would be the ones to not implement OIDC according
    // to the standard, for now I'll only support personal microsoft accounts
    BaseOidcProviderBuilder::new(
        "https://login.microsoftonline.com/9188040d-6c67-4c5b-b112-36a304b66dad/v2.0",
        &format!("{}/api/auth/microsoft/callback", get_backend_domain()),
        &client_id,
        &client_secret,
    )
    .with_prompt(CoreAuthPrompt::Consent)
    .build()
}
