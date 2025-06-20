use serde::{Deserialize, Deserializer, de::Error};

pub fn disallow_empty_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let input_string: Option<String> = Option::deserialize(deserializer)?;

    let Some(query_string) = input_string else {
        return Err(Error::custom("field is not a string"));
    };

    if query_string.is_empty() {
        return Err(Error::custom("field is empty"));
    }

    Ok(query_string)
}
