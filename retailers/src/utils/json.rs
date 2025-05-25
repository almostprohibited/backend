use serde_json::Value;
use tracing::error;

use crate::errors::RetailerError;

pub(crate) fn json_get_object(object: &Value, key: String) -> Result<&Value, RetailerError> {
    let Some(value) = object.get(&key) else {
        let message = format!("JSON is missing '{}'", key);

        error!(message);

        return Err(RetailerError::ApiResponseMissingKey(message));
    };

    Ok(value)
}

pub(crate) fn json_get_array(object: &Value) -> Result<&Vec<Value>, RetailerError> {
    let Some(value) = object.as_array() else {
        let message = format!("JSON prop is not an array");

        error!(message);

        return Err(RetailerError::GeneralError(message));
    };

    Ok(value)
}
