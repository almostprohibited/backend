use std::{env, sync::LazyLock};

use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use ed25519_dalek::{SigningKey, pkcs8::DecodePrivateKey};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const PRIVATE_KEY_ENV: &str = "PRIVATE_KEY";

#[cfg(not(test))]
pub(super) static PRIVATE_KEY: LazyLock<SigningKey> = LazyLock::new(|| {
    let b64_pem = env::var(PRIVATE_KEY_ENV).expect("{PRIVATE_KEY_ENV} to be passed");

    let pem = String::from_utf8(STANDARD.decode(b64_pem).expect("Valid base64 private key"))
        .expect("{PRIVATE_KEY_ENV} to be formatted correctly");

    SigningKey::from_pkcs8_pem(&pem).expect("Valid private key")
});

#[cfg(test)]
pub(super) static PRIVATE_KEY: LazyLock<SigningKey> =
    LazyLock::new(|| SigningKey::from_bytes(&[4u8; 32]));

static ENCODED_PUBLIC_KEY: LazyLock<String> =
    LazyLock::new(|| URL_SAFE_NO_PAD.encode(PRIVATE_KEY.verifying_key().as_bytes()));

pub static JWK_KID: LazyLock<String> = LazyLock::new(|| {
    let json = format!(
        r#"{{"crv":"Ed25519","kty":"OKP","x":"{}"}}"#,
        *ENCODED_PUBLIC_KEY
    );

    URL_SAFE_NO_PAD.encode(Sha256::digest(json.as_bytes()))
});

pub fn get_jwk_keys() -> Value {
    json!({
       "keys": [
           {
               "kty": "OKP",
               "crv": "Ed25519",
               "x": *ENCODED_PUBLIC_KEY,
               "kid": *JWK_KID,
           }
       ]
    })
}
