use std::cmp;

use regex::Regex;

use crate::errors::RetailerError;

const TRUNCATE_ERR_LENGTH: usize = 20;

pub(crate) fn unwrap_regex_capture(regex: &Regex, haystack: &str) -> Result<String, RetailerError> {
    let Some(captures) = regex.captures(haystack) else {
        let mut cleaned_haystack = haystack.replace("\n", "");
        let _ = cleaned_haystack.split_off(cmp::min(cleaned_haystack.len(), TRUNCATE_ERR_LENGTH));

        return Err(RetailerError::GeneralError(format!(
            "Failed to search for {} inside of {}",
            regex.as_str(),
            cleaned_haystack
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
