use std::time::Duration;

use indexmap::IndexMap;
use web_bot_auth::{
    components::{CoveredComponent, DerivedComponent, HTTPField, HTTPFieldParametersSet},
    keyring::Algorithm,
    message_signatures::{MessageSigner, SignedMessage, UnsignedMessage},
};

use crate::{
    http_sig::{JWK_KID, jwk::PRIVATE_KEY},
    string_utils::{generate_random_string, get_domain},
    utils::is_beta_environment,
};

const SIG_EXPIRY_SECONDS: u64 = 60 * 60;
const KEY: &str = "sig1";

#[derive(Debug)]
pub struct HttpSignatureHeaders {
    host: String,
    is_directory: bool,
    pub signature: String,
    pub signature_input: String,
    pub signature_agent: Option<String>,
}

impl HttpSignatureHeaders {
    // TODO: write builder for this to get rid of constructor
    pub fn new(host: &str, is_directory: bool, signature_agent: Option<String>) -> Self {
        Self {
            host: host.into(),
            is_directory,
            signature: Default::default(),
            signature_input: Default::default(),
            // NOTE: shopify may still be checking the legacy format
            // (dumb, why would they enforce checks on an I-D, active or not)
            // https://datatracker.ietf.org/doc/html/draft-ietf-webbotauth-httpsig-protocol-01#name-legacy-signature-agent-sf-st
            signature_agent: signature_agent
                .and_then(|sig_agent| Some(format!(r#""{sig_agent}""#))),
            // signature_agent: signature_agent.and_then(|sig_agent| {
            //     Some(format!(
            //         r#"{}="{}/.well-known/http-message-signatures-directory""#,
            //         KEY,
            //         sig_agent.strip_suffix("/").unwrap_or(&sig_agent)
            //     ))
            // }),
        }
    }
}

impl SignedMessage for HttpSignatureHeaders {
    fn lookup_component(&self, name: &CoveredComponent) -> Vec<String> {
        match name {
            CoveredComponent::Derived(DerivedComponent::Authority { .. }) => {
                vec![self.host.clone()]
            }
            CoveredComponent::HTTP(HTTPField { name, .. }) if name == "signature" => {
                vec![self.signature.clone()]
            }
            CoveredComponent::HTTP(HTTPField { name, .. }) if name == "signature-input" => {
                vec![self.signature_input.clone()]
            }
            CoveredComponent::HTTP(HTTPField { name, .. }) if name == "signature-agent" => {
                let Some(sig_agent) = &self.signature_agent else {
                    return vec![];
                };

                vec![sig_agent.clone()]
            }
            _ => vec![],
        }
    }
}

impl UnsignedMessage for HttpSignatureHeaders {
    fn fetch_components_to_cover(&self) -> IndexMap<CoveredComponent, String> {
        let mut components = IndexMap::from_iter([(
            CoveredComponent::Derived(DerivedComponent::Authority {
                req: self.is_directory,
            }),
            self.host.clone(),
        )]);

        if let Some(signature_agent) = self.signature_agent.clone() {
            components.insert(
                CoveredComponent::HTTP(HTTPField {
                    name: "signature-agent".to_string(),
                    // parameters: HTTPFieldParametersSet(vec![HTTPFieldParameters::Key(
                    //     KEY.to_string(),
                    // )]),
                    parameters: HTTPFieldParametersSet(vec![]),
                }),
                signature_agent,
            );
        }

        components
    }

    fn register_header_contents(&mut self, signature_input: String, signature_header: String) {
        self.signature_input = format!("{KEY}={signature_input}");
        self.signature = format!("{KEY}={signature_header}");
    }
}

fn get_signing_identity() -> String {
    if is_beta_environment() {
        return "https://beta.almostprohibited.ca".into();
    }

    "https://almostprohibited.ca".into()
}

// TODO: implement Content-Digest header
// https://datatracker.ietf.org/doc/html/draft-ietf-webbotauth-httpsig-protocol-01#name-possession-proof-on-the-dir
pub fn create_directory_headers(host: &str) -> HttpSignatureHeaders {
    let signer = MessageSigner {
        keyid: JWK_KID.clone(),
        nonce: generate_random_string(24),
        tag: "http-message-signatures-directory".into(),
    };

    let mut headers = HttpSignatureHeaders::new(host, true, None);

    signer
        .generate_signature_headers_content(
            &mut headers,
            Duration::from_secs(SIG_EXPIRY_SECONDS),
            Algorithm::Ed25519,
            &PRIVATE_KEY.to_bytes(),
        )
        .unwrap();

    headers
}

pub fn create_request_headers(request_url: &str) -> HttpSignatureHeaders {
    let signer = MessageSigner {
        keyid: JWK_KID.clone(),
        nonce: generate_random_string(24),
        tag: "web-bot-auth".into(),
    };

    let mut headers = HttpSignatureHeaders::new(
        &get_domain(request_url),
        false,
        Some(get_signing_identity()),
    );

    signer
        .generate_signature_headers_content(
            &mut headers,
            Duration::from_secs(SIG_EXPIRY_SECONDS),
            Algorithm::Ed25519,
            &PRIVATE_KEY.to_bytes(),
        )
        .unwrap();

    headers
}

#[cfg(test)]
mod test {
    use web_bot_auth::{
        SignatureAgentLink, WebBotAuthVerifier,
        components::{CoveredComponent::Derived, DerivedComponent::Authority},
        keyring::{JSONWebKeySet, KeyRing},
    };

    use crate::{
        http_sig::{create_request_headers, jwk::get_jwk_keys, web_sig::get_signing_identity},
        string_utils::get_domain,
    };

    #[test]
    fn test_message_sign() {
        let url = "https://example.com";

        let message = create_request_headers(url);
        let verifier = WebBotAuthVerifier::parse(&message).unwrap();
        let agents = verifier.get_signature_agents();

        assert_eq!(agents.len(), 1);
        assert!(
            matches!(agents.first().unwrap(), SignatureAgentLink::LegacyDirectory(domain) if domain.starts_with(&get_signing_identity()))
        );

        let base = verifier.get_parsed_label().base.clone();
        let security = base.parameters.details.possibly_insecure(|_| false);

        assert_eq!(
            base.components
                .get(&Derived(Authority { req: false }))
                .unwrap(),
            &get_domain(url)
        );
        assert_eq!(base.parameters.details.tag.clone().unwrap(), "web-bot-auth");

        assert!(!security.is_expired.unwrap_or(true));
        assert!(!security.nonce_is_invalid.unwrap_or(true));

        let mut keyring = KeyRing::default();
        let jwt: JSONWebKeySet = serde_json::from_value(get_jwk_keys().clone()).unwrap();
        keyring.import_jwks(jwt);

        assert!(verifier.verify(&keyring, None).is_ok());
    }
}
