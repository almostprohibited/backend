use regex::Regex;

use crate::errors::RetailerError;

pub(crate) fn unwrap_regex_capture(regex: &Regex, haystack: &str) -> Result<String, RetailerError> {
    let Some(captures) = regex.captures(haystack) else {
        return Err(RetailerError::GeneralError(format!(
            "Failed to search for {} inside of {}",
            regex.as_str(),
            haystack
        )));
    };

    let Some(result) = captures.get(1) else {
        return Err(RetailerError::GeneralError(format!(
            "Invalid return capture group (should not be possible) for {}",
            regex.as_str()
        )));
    };

    Ok(result.as_str().to_string())
}
