use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct Message {
    pub body: String,
    pub ip_address: String,
    pub timestamp: u64,
    pub subject: Option<String>,
    pub email: Option<String>,
}

impl Message {
    pub fn new(
        body: String,
        ip_address: String,
        subject: Option<String>,
        email: Option<String>,
    ) -> Self {
        Self {
            body,
            ip_address,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap() // this should not fail since the current time is always > UNIX_EPOCH
                .as_secs(),
            subject,
            email,
        }
    }
}
