use std::time::{SystemTime, UNIX_EPOCH};

pub fn get_current_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap() // this should not fail since the current time is always > UNIX_EPOCH
        .as_secs()
}
