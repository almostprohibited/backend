use mongodb::bson::DateTime;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};

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

pub fn from_u64_seconds_to_datetime<S>(timestamp: &u64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let datetime = DateTime::from_millis(*timestamp as i64 * 1000);

    datetime.serialize(serializer)
}

pub fn from_datetime_to_u64_seconds<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let input: Option<DateTime> = Option::deserialize(deserializer)?;

    let Some(input) = input else {
        return Err(Error::custom("field is not a BSON DateTime"));
    };

    // goodbye milliseconds
    Ok((input.timestamp_millis() / 1000) as u64)
}
