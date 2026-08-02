use std::{env, str::FromStr, sync::LazyLock, time::Duration};

use ed25519_dalek::{SigningKey, pkcs8::DecodePrivateKey};
use indexmap::IndexMap;
use reqwest::Url;
use serde_json::{Value, json};
use web_bot_auth::{
    components::{CoveredComponent, DerivedComponent, HTTPField, HTTPFieldParametersSet},
    keyring::Algorithm,
    message_signatures::{MessageSigner, UnsignedMessage},
};

use crate::{string_utils::generate_random_string, utils::is_beta_environment};

const PRIVATE_KEY_ENV: &str = "PRIVATE_KEY";

// TODO: don't hard code these
pub const JWK_KID: &str = "HE08axWirsRZyAyvEiiHJ_oQdyzGtSEehXho3SF5q5c";
pub const JWK_KEYS: LazyLock<Value> = LazyLock::new(|| {
    json!({
       "keys": [
           {
               "kty": "OKP",
               "crv": "Ed25519",
               "x": "mkXzzFb6KUbe-5Crh8At9Ptv8HYsuFSoOehjqy2S27c"
           }
       ]
    })
});

#[derive(Debug)]
pub struct HttpSignatureHeaders {
    host: String,
    is_directory: bool,
    pub signature: String,
    pub signature_input: String,
    /// Yields double quote formatted string
    pub signature_agent: Option<String>,
}

impl HttpSignatureHeaders {
    // TODO: write builder for this to get rid of constructor
    /// Pass `signature_agent` as non quoted string if supplying agent
    pub fn new(host: &str, is_directory: bool, signature_agent: Option<String>) -> Self {
        Self {
            host: host.into(),
            is_directory,
            signature: Default::default(),
            signature_input: Default::default(),
            signature_agent: signature_agent.and_then(|sig_agent| Some(format!("\"{sig_agent}\""))),
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
                    parameters: HTTPFieldParametersSet(vec![]),
                }),
                signature_agent,
            );
        }

        components
    }

    fn register_header_contents(&mut self, signature_input: String, signature_header: String) {
        self.signature_input = format!("sig1={signature_input}");
        self.signature = format!("sig1={signature_header}");
    }
}

fn get_signing_key() -> [u8; 32] {
    let pem = env::var(PRIVATE_KEY_ENV).expect("{PRIVATE_KEY_ENV} to be passed");

    SigningKey::from_pkcs8_pem(&pem)
        .expect("{PRIVATE_KEY_ENV} to be formatted correctly")
        .to_bytes()
}

fn get_signing_identity() -> String {
    if is_beta_environment() {
        return "https://beta.almostprohibited.ca".into();
    }

    "https://almostprohibited.ca".into()
}

// """should""" is doing a lot of heavy lifting in this section here
// https://datatracker.ietf.org/doc/html/draft-meunier-http-message-signatures-directory-03#name-binding-keys-to-the-directo
pub fn create_directory_headers(host: &str) -> HttpSignatureHeaders {
    let signer = MessageSigner {
        keyid: JWK_KID.into(),
        nonce: generate_random_string(24),
        tag: "http-message-signatures-directory".into(),
    };

    let mut headers = HttpSignatureHeaders::new(host, true, None);

    signer
        .generate_signature_headers_content(
            &mut headers,
            Duration::from_secs(10),
            Algorithm::Ed25519,
            &get_signing_key(),
        )
        .unwrap();

    headers
}

pub fn create_request_headers(request_url: &str) -> HttpSignatureHeaders {
    let signer = MessageSigner {
        keyid: JWK_KID.into(),
        nonce: generate_random_string(24),
        tag: "web-bot-auth".into(),
    };

    let parsed_url = Url::from_str(request_url).unwrap();
    let host = parsed_url.host_str().unwrap();

    let mut headers = HttpSignatureHeaders::new(host, false, Some(get_signing_identity()));

    signer
        .generate_signature_headers_content(
            &mut headers,
            Duration::from_secs(120),
            Algorithm::Ed25519,
            &get_signing_key(),
        )
        .unwrap();

    headers
}
