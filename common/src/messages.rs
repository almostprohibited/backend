use serde::Serialize;

use crate::utils::get_current_time;

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
            timestamp: get_current_time(),
            subject,
            email,
        }
    }
}
