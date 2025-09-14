use serde_json::Value;

use crate::errors::RetailerError;

pub(crate) fn json_get_object(object: &Value, key: String) -> Result<&Value, RetailerError> {
    let Some(value) = object.get(&key) else {
        return Err(RetailerError::ApiResponseMissingKey(format!(
            "JSON is missing '{key}'"
        )));
    };

    Ok(value)
}

pub(crate) fn json_get_array(object: &Value) -> Result<&Vec<Value>, RetailerError> {
    let Some(value) = object.as_array() else {
        return Err(RetailerError::GeneralError(
            "JSON prop is not an array".into(),
        ));
    };

    Ok(value)
}
