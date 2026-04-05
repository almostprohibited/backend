use base64::{Engine, prelude::BASE64_STANDARD};
use rand::SeedableRng;
use rand::distr::{Alphanumeric, SampleString};
use rand::rngs::{StdRng, SysRng};
use sha2::{Digest, Sha256};

pub fn generate_random_string(len: u64) -> String {
    let mut rng = StdRng::try_from_rng(&mut SysRng).unwrap();

    Alphanumeric.sample_string(&mut rng, len as usize)
}

pub fn sha256_hash_string(token: &str) -> String {
    let hash = Sha256::digest(token.as_bytes());
    BASE64_STANDARD.encode(&hash)
}
