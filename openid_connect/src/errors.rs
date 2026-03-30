use openidconnect::{
    ClaimsVerificationError, ConfigurationError, DiscoveryError, HttpClientError, reqwest::Error,
    url::ParseError,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OidcError {
    #[error("Failed to perform automated oauth2 discovery")]
    DiscoveryError(#[from] DiscoveryError<HttpClientError<Error>>),
    #[error("Failed to create URL properties")]
    ParseError(#[from] ParseError),
    #[error("Failed to create token exchange request")]
    ConfigurationError(#[from] ConfigurationError),
    #[error("Failed to exchange code into token: {0}")]
    TokenExchangeError(String),
    #[error("Missing ID token in exchange response")]
    MissingIdTokenError,
    #[error("Failed to validate claims using keys")]
    InvalidTokenClaimsError(#[from] ClaimsVerificationError),
    #[error("Exchanged token missing nonce")]
    MissingClaimNonceError,
    #[error("Local store missing nonce")]
    MissingNonceError,
    #[error("Provider returned nonce does not match local")]
    NonceMismatchError,
}
