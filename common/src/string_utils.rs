use base64::{Engine, prelude::BASE64_STANDARD};
use rand::distr::{Alphanumeric, SampleString};
use rand::rngs::{StdRng, SysRng};
use rand::{RngExt, SeedableRng};
use sha2::{Digest, Sha256};

const OTP_LENGTH: usize = 6;

pub fn generate_random_string(len: u64) -> String {
    // TODO: lazy static the stdrng call for performance reasons
    let mut rng = StdRng::try_from_rng(&mut SysRng).unwrap();

    Alphanumeric.sample_string(&mut rng, len as usize)
}

pub fn generate_random_code() -> String {
    let mut rng = StdRng::try_from_rng(&mut SysRng).unwrap();

    let random_number = rng.random_range(0..10u32.pow(OTP_LENGTH as u32));

    format!("{random_number:0>OTP_LENGTH$}")
}

pub fn sha256_hash_string(token: &str) -> String {
    let hash = Sha256::digest(token.as_bytes());
    BASE64_STANDARD.encode(&hash)
}
