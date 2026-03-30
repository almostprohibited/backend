use crate::utils::is_beta_environment;

const USER_AGENT: &str =
    "almostprohibited/1.0 (+https://almostprohibited.ca/contact/; hello@almostprohibited.ca)";

const DEV_USER_AGENT: &str = "almostprohibited/1.0 (+https://almostprohibited.ca/contact/; +development; hello@almostprohibited.ca)";

pub fn get_user_agent() -> String {
    match is_beta_environment() {
        true => DEV_USER_AGENT,
        false => USER_AGENT,
    }
    .to_string()
}
