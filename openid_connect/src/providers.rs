use std::env;

use common::constants::{
    OIDC_CLIENT_ID_DISCORD, OIDC_CLIENT_ID_GOOGLE, OIDC_CLIENT_ID_MICROSOFT,
    OIDC_CLIENT_SECRET_DISCORD, OIDC_CLIENT_SECRET_GOOGLE, OIDC_CLIENT_SECRET_MICROSOFT,
    OIDC_TENANT_ID_MICROSOFT,
};
use openidconnect::core::CoreAuthPrompt;

use crate::base_provider::{BaseOidcProvider, BaseOidcProviderBuilder};

pub fn get_discord_oidc_provider(callback_url: &str, _scopes: Vec<String>) -> BaseOidcProvider {
    let client_id = env::var(OIDC_CLIENT_ID_DISCORD).unwrap().to_string();
    let client_secret = env::var(OIDC_CLIENT_SECRET_DISCORD).unwrap().to_string();

    BaseOidcProviderBuilder::new(
        "https://discord.com",
        callback_url,
        &client_id,
        &client_secret,
    )
    .with_authorization_url("https://discord.com/oauth2/authorize")
    .build()
}

pub fn get_google_oidc_provider(callback_url: &str, scopes: Vec<String>) -> BaseOidcProvider {
    let client_id = env::var(OIDC_CLIENT_ID_GOOGLE).unwrap().to_string();
    let client_secret = env::var(OIDC_CLIENT_SECRET_GOOGLE).unwrap().to_string();

    BaseOidcProviderBuilder::new(
        "https://accounts.google.com",
        callback_url,
        &client_id,
        &client_secret,
    )
    .with_prompt(CoreAuthPrompt::Consent)
    .with_scopes(scopes)
    .build()
}

pub fn get_microsoft_oidc_provider(callback_url: &str, scopes: Vec<String>) -> BaseOidcProvider {
    let client_id = env::var(OIDC_CLIENT_ID_MICROSOFT).unwrap().to_string();
    let client_secret = env::var(OIDC_CLIENT_SECRET_MICROSOFT).unwrap().to_string();
    let tenant_id = env::var(OIDC_TENANT_ID_MICROSOFT).unwrap().to_string();

    // of course microsoft would be the ones to not implement OIDC according
    // to the standard, for now I'll only support personal microsoft accounts
    BaseOidcProviderBuilder::new(
        &format!("https://login.microsoftonline.com/{tenant_id}/v2.0"),
        callback_url,
        &client_id,
        &client_secret,
    )
    .with_prompt(CoreAuthPrompt::Consent)
    .with_scopes(scopes)
    .build()
}
