use openidconnect::{
    AuthUrl, AuthenticationFlow, AuthorizationCode, Client, ClientId, ClientSecret, CsrfToken,
    EmptyAdditionalClaims, EmptyExtraTokenFields, EndpointMaybeSet, EndpointNotSet, EndpointSet,
    IdTokenFields, IssuerUrl, Nonce, RedirectUrl, RevocationErrorResponseType, Scope,
    StandardErrorResponse, StandardTokenIntrospectionResponse, StandardTokenResponse,
    core::{
        CoreAuthDisplay, CoreAuthPrompt, CoreClient, CoreErrorResponseType, CoreGenderClaim,
        CoreJsonWebKey, CoreJweContentEncryptionAlgorithm, CoreJwsSigningAlgorithm,
        CoreProviderMetadata, CoreResponseType, CoreRevocableToken, CoreTokenType,
    },
};
use tracing::{debug, error};

use crate::{errors::OidcError, traits::OidcAuthorizationProps, utils::get_reqwest_client};

// typing hell
type OidcClient = Client<
    EmptyAdditionalClaims,
    CoreAuthDisplay,
    CoreGenderClaim,
    CoreJweContentEncryptionAlgorithm,
    CoreJsonWebKey,
    CoreAuthPrompt,
    StandardErrorResponse<CoreErrorResponseType>,
    StandardTokenResponse<
        IdTokenFields<
            EmptyAdditionalClaims,
            EmptyExtraTokenFields,
            CoreGenderClaim,
            CoreJweContentEncryptionAlgorithm,
            CoreJwsSigningAlgorithm,
        >,
        CoreTokenType,
    >,
    StandardTokenIntrospectionResponse<EmptyExtraTokenFields, CoreTokenType>,
    CoreRevocableToken,
    StandardErrorResponse<RevocationErrorResponseType>,
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointMaybeSet,
    EndpointMaybeSet,
>;

pub struct BaseOidcProvider {
    provider_url: String,
    redirect_url: String,
    client_id: String,
    client_secret: String,
    authorization_url: Option<String>,
    prompt: Option<CoreAuthPrompt>,
    scopes: Option<Vec<String>>,
}

#[derive(Debug)]
pub struct OidcClaims {
    pub id: String,
    pub email: Option<String>,
}

pub(crate) struct BaseOidcProviderBuilder {
    provider_url: String,
    redirect_url: String,
    client_id: String,
    client_secret: String,
    authorization_url: Option<String>,
    prompt: Option<CoreAuthPrompt>,
    scopes: Option<Vec<String>>,
}

impl BaseOidcProvider {
    async fn get_oidc_client(&self) -> Result<OidcClient, OidcError> {
        let client_id = ClientId::new(self.client_id.clone());
        let client_secret = ClientSecret::new(self.client_secret.clone());

        let redirect_url = RedirectUrl::new(self.redirect_url.clone())?;
        let issuer_url = IssuerUrl::new(self.provider_url.clone())?;

        let mut provider_metadata =
            CoreProviderMetadata::discover_async(issuer_url, get_reqwest_client()).await?;

        if let Some(authorization_url) = &self.authorization_url {
            provider_metadata = provider_metadata
                .set_authorization_endpoint(AuthUrl::new(authorization_url.clone())?);
        }

        Ok(
            CoreClient::from_provider_metadata(provider_metadata, client_id, Some(client_secret))
                .set_redirect_uri(redirect_url),
        )
    }

    pub async fn fetch_authorization_url(&self) -> Result<OidcAuthorizationProps, OidcError> {
        let client = self.get_oidc_client().await?;

        let mut authorization_url_builder = client.authorize_url(
            AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        );

        if let Some(prompt) = &self.prompt {
            authorization_url_builder = authorization_url_builder.add_prompt(prompt.clone());
        }

        if let Some(string_scopes) = &self.scopes
            && string_scopes.len() > 0
        {
            let scopes: Vec<Scope> = string_scopes
                .iter()
                .map(|scope| Scope::new(scope.clone()))
                .collect();

            authorization_url_builder = authorization_url_builder.add_scopes(scopes);
        }

        let (auth_url, csrf, nonce) = authorization_url_builder.url();

        debug!("Returning auth url: {auth_url:?}");

        // unwrap and return raw values instead of passing
        // OIDC objects to caller
        Ok(OidcAuthorizationProps {
            url: auth_url.to_string(),
            csrf: csrf.secret().clone(),
            nonce: nonce.secret().clone(),
        })
    }

    pub async fn exchange_code(&self, code: &str, nonce: &str) -> Result<OidcClaims, OidcError> {
        let client = self.get_oidc_client().await?;

        let response = client
            .exchange_code(AuthorizationCode::new(code.to_string()))?
            .request_async(get_reqwest_client())
            .await
            .map_err(|err| OidcError::TokenExchangeError(err.to_string()))?;

        let verifier = client.id_token_verifier();

        let Some(id_token) = response.extra_fields().id_token() else {
            return Err(OidcError::MissingIdTokenError);
        };

        let claims = id_token.claims(&verifier, &Nonce::new(nonce.to_string()))?;

        debug!("{claims:?}");

        let claimed_nonce = match claims.nonce() {
            Some(claim) => claim.secret(),
            None => return Err(OidcError::MissingClaimNonceError),
        };

        if *claimed_nonce != nonce {
            error!("Returned nonces do not match");
            return Err(OidcError::NonceMismatchError);
        }

        let id = claims.subject().to_string();
        let email = claims.email().map(|user_email| user_email.to_string());

        Ok(OidcClaims { id, email })
    }
}

impl BaseOidcProviderBuilder {
    pub(crate) fn new(
        provider_url: &str,
        redirect_url: &str,
        client_id: &str,
        client_secret: &str,
    ) -> Self {
        Self {
            provider_url: provider_url.to_string(),
            redirect_url: redirect_url.to_string(),
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            authorization_url: None,
            prompt: None,
            scopes: None,
        }
    }

    pub(crate) fn with_authorization_url(mut self, authorization_url: &str) -> Self {
        self.authorization_url = Some(authorization_url.to_string());

        self
    }

    pub(crate) fn with_prompt(mut self, prompt: CoreAuthPrompt) -> Self {
        self.prompt = Some(prompt);

        self
    }

    pub(crate) fn with_scopes(mut self, scopes: Vec<String>) -> Self {
        self.scopes = Some(scopes);

        self
    }

    pub(crate) fn build(self) -> BaseOidcProvider {
        BaseOidcProvider {
            provider_url: self.provider_url,
            redirect_url: self.redirect_url,
            client_id: self.client_id,
            client_secret: self.client_secret,
            authorization_url: self.authorization_url,
            prompt: self.prompt,
            scopes: self.scopes,
        }
    }
}
