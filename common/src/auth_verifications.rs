use crate::serde_utils::{from_datetime_to_u64_seconds, from_u64_seconds_to_datetime};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct AuthVerification {
    pub hashed_code: String,
    #[serde(serialize_with = "from_u64_seconds_to_datetime")]
    #[serde(deserialize_with = "from_datetime_to_u64_seconds")]
    pub timestamp: u64,
    pub ip_addr: Option<String>,
    pub nonce: Option<String>,
}
