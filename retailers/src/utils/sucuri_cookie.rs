use std::sync::LazyLock;

use crawler::{request::RequestBuilder, unprotected::UnprotectedCrawler};
use regex::Regex;
use tracing::{trace, warn};

use crate::{errors::RetailerError, utils::regex::unwrap_regex_capture};

use base64::{Engine, prelude::BASE64_STANDARD};

static MAIN_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\bS\s*=\s*'([^']*)'").expect("Regex should compile as nothing has changed")
});

static COOKIE_NAME_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r##";document\.cookie=(.*?)\+\s*\"=\"\s*\+"##)
        .expect("Regex should compile as nothing has changed")
});

static COOKIE_VALUE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"=(.*?)\s+\+\s+'';").expect("Regex should compile as nothing has changed")
});

static STRING_CHAR_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"String\.fromCharCode\((\d+)\)")
        .expect("Regex should compile as nothing has changed")
});

// SucURI's wordpress "firewall" might as well not be there
// below is cursed Javascript to Rust translation code
// (I don't want to explore Deno)
pub(crate) async fn get_sucuri_cookie(home_page: &str) -> Result<String, RetailerError> {
    let request = RequestBuilder::new().set_url(home_page).build();
    let result = UnprotectedCrawler::make_web_request(request).await?;

    if result.response_code.is_success() {
        warn!("Page did not redirect to check page");

        return Ok(String::default());
    }

    let base64 = unwrap_regex_capture(&MAIN_REGEX, &result.body)?;

    trace!("{base64}");

    let Ok(decoded_base64) = BASE64_STANDARD.decode(&base64) else {
        return Err(RetailerError::GeneralError(format!(
            "Failed to decode base64, got this instead: {base64}"
        )));
    };

    let Ok(decoded_string) = String::from_utf8(decoded_base64) else {
        return Err(RetailerError::GeneralError(
            "Invalid string, decoded base64 did not convert into a string".to_string(),
        ));
    };

    let cookie_name = get_cookie_name(&decoded_string)?;
    let cookie_value = get_cookie_value(&decoded_string)?;

    Ok(format!("{cookie_name}={cookie_value};"))
}

fn get_cookie_name(haystack: &str) -> Result<String, RetailerError> {
    let cookie_name_obfuscated = unwrap_regex_capture(&COOKIE_NAME_REGEX, haystack)?;
    let mut cookie_name_parts: Vec<String> = Vec::new();

    for cooke_name_part in cookie_name_obfuscated.split("+") {
        let Some(individual_char) = cooke_name_part.get(1..2) else {
            return Err(RetailerError::GeneralError(format!(
                "Failed to map value: {cooke_name_part}"
            )));
        };

        cookie_name_parts.push(individual_char.to_string());
    }

    Ok(cookie_name_parts.join(""))
}

fn get_cookie_value(haystack: &str) -> Result<String, RetailerError> {
    // the JS starts with `i=<string parts>;cookie`
    // I want the inside parts
    let cookie_value_obfuscated = unwrap_regex_capture(&COOKIE_VALUE_REGEX, haystack)?;

    let mut reconstructed_parts: Vec<String> = Vec::new();

    let char_code_parts: Vec<&str> = cookie_value_obfuscated.split(" + ").collect();

    for part in char_code_parts {
        let Ok(char_code) = unwrap_regex_capture(&STRING_CHAR_REGEX, part) else {
            let Some(individual_char) = part.get(1..2) else {
                return Err(RetailerError::GeneralError(format!(
                    "Captured non String.fromCharCode, but failed to map to char: {part}"
                )));
            };

            reconstructed_parts.push(individual_char.to_string());
            continue;
        };

        let Ok(char_code) = char_code.parse::<u32>() else {
            return Err(RetailerError::GeneralError(format!(
                "Char code is not a number: {char_code}"
            )));
        };

        let Some(parsed_char) = char::from_u32(char_code) else {
            return Err(RetailerError::GeneralError(format!(
                "Failed to convert char into valid UTF-8: {char_code}"
            )));
        };

        reconstructed_parts.push(parsed_char.to_string());
    }

    Ok(reconstructed_parts.join(""))
}
