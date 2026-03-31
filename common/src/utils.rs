use std::{
    env,
    time::{SystemTime, UNIX_EPOCH},
};

use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Deserializer, de::Error};

const ONE_DAY_MINUTES: i64 = 1440;

pub fn get_current_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap() // this should not fail since the current time is always > UNIX_EPOCH
        .as_secs()
}

pub fn is_beta_environment() -> bool {
    env::var("STAGE").unwrap_or("beta".to_string()) == "beta"
}

/// Returns the UNIX timestamp of whatever `today - delta_days`
/// is, normalized to the start of the day
pub fn normalized_relative_days(delta_days: i64) -> i64 {
    let past_days: i64 = delta_days * ONE_DAY_MINUTES * 60;

    let current_time = get_current_time() as i64;
    let offset_time = current_time - past_days;

    (offset_time / ONE_DAY_MINUTES) * ONE_DAY_MINUTES
}

pub(crate) fn object_id_to_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Option<ObjectId> = Option::deserialize(deserializer)?;

    let Some(object_id) = value else {
        return Err(Error::custom("field is not MongoDB ObjectId"));
    };

    Ok(Some(object_id.to_string()))
}

fn get_domain() -> String {
    let mut domain_parts = vec!["ca", "almostprohibited"];

    if is_beta_environment() {
        domain_parts.push("beta");
    }

    domain_parts.reverse();

    format!("https://{}", domain_parts.join("."))
}

pub fn get_frontend_domain() -> String {
    if cfg!(debug_assertions) {
        return "http://localhost:3000".to_string();
    }

    get_domain()
}

pub fn get_backend_domain() -> String {
    if cfg!(debug_assertions) {
        return "http://localhost:3001".to_string();
    }

    get_domain()
}
