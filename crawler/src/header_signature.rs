use std::{env, time::Duration};

use common::string_utils::generate_random_string;
use ed25519_dalek::{SigningKey, pkcs8::DecodePrivateKey};
use indexmap::IndexMap;
use web_bot_auth::{
    components::{CoveredComponent, DerivedComponent, HTTPField, HTTPFieldParametersSet},
    keyring::Algorithm,
    message_signatures::{MessageSigner, UnsignedMessage},
};

#[derive(Default, Debug)]
struct UnsignedMessageHeaders {
    signature: String,
    signature_input: String,
}

impl UnsignedMessage for UnsignedMessageHeaders {
    fn fetch_components_to_cover(&self) -> IndexMap<CoveredComponent, String> {
        IndexMap::from_iter([
            (
                CoveredComponent::Derived(DerivedComponent::Authority { req: false }),
                "example.com".to_string(),
            ),
            (
                CoveredComponent::HTTP(HTTPField {
                    name: "signature-agent".to_string(),
                    parameters: HTTPFieldParametersSet(vec![]),
                }),
                r#""https://beta.almostprohibitited.ca""#.to_string(),
            ),
        ])
    }

    fn register_header_contents(&mut self, signature_input: String, signature_header: String) {
        self.signature_input = format!("sig1={signature_input}");
        self.signature = format!("sig1={signature_header}");
    }
}

fn main() {
    let pem = env::var("PRIVATE_KEY").expect("PRIVATE_KEY to be passed");
    let key_id = env::var("JWK_KID").expect("JWK_KID to be passed");

    let key = SigningKey::from_pkcs8_pem(&pem)
        .expect("PRIVATE_KEY to be formatted correctly")
        .to_bytes();

    let signer = MessageSigner {
        keyid: key_id,
        nonce: generate_random_string(24),
        tag: "web-bot-auth".into(),
    };

    let mut headers = UnsignedMessageHeaders::default();

    signer
        .generate_signature_headers_content(
            &mut headers,
            Duration::from_secs(120),
            Algorithm::Ed25519,
            &key,
        )
        .unwrap();

    println!("{}\n{}", headers.signature, headers.signature_input);
}
