use std::{
    env,
    time::{SystemTime, UNIX_EPOCH},
};

const ONE_DAY_MINUTES: i64 = 1440;

pub fn get_current_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap() // this should not fail since the current time is always > UNIX_EPOCH
        .as_secs()
}

pub fn is_beta_environment() -> bool {
    env::var("STAGE").unwrap_or_default() == "beta"
}

/// Returns the UNIX timestamp of whatever `today - delta_days`
/// is, normalized to the start of the day
pub fn normalized_relative_days(delta_days: i64) -> i64 {
    let past_days: i64 = delta_days * ONE_DAY_MINUTES * 60;

    let current_time = get_current_time() as i64;
    let offset_time = current_time - past_days;

    (offset_time / ONE_DAY_MINUTES) * ONE_DAY_MINUTES
}
