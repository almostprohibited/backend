use crate::deserialize_disallow_empty_string::disallow_empty_string;
use crate::result::base::CrawlResult;
use crate::result::enums::Category;
use crate::result::enums::RetailerName;

use mongodb::bson::doc;
use serde::Deserialize;
use serde::Deserializer;
use serde::de::Error;
use serde_with::NoneAsEmptyString;
use serde_with::serde_as;
use std::str::FromStr;
use strum_macros::EnumString;
use tracing::debug;

pub struct CollectionSearchResults {
    pub items: Vec<CrawlResult>,
    pub total_count: u64,
}

impl Default for CollectionSearchResults {
    fn default() -> Self {
        Self::new()
    }
}

impl CollectionSearchResults {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            total_count: 0,
        }
    }
}

#[serde_as]
#[derive(Debug, Default, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct ApiSearchInput {
    #[serde(deserialize_with = "disallow_empty_string")]
    pub query: String,
    #[serde_as(as = "NoneAsEmptyString")]
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(deserialize_with = "string_to_cents")]
    #[serde(default)]
    pub min_price: Option<u32>,
    #[serde(deserialize_with = "string_to_cents")]
    #[serde(default)]
    pub max_price: Option<u32>,
    #[serde(default)]
    pub sort: Sort,
    #[serde(default)]
    pub category: Category,
    #[serde(deserialize_with = "string_to_retailer_array")]
    #[serde(default)]
    pub retailers: Vec<RetailerName>,
}

#[derive(Debug, Default, Deserialize, EnumString, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum Sort {
    #[default]
    Relevant,
    PriceAsc,
    PriceDesc,
}

fn string_to_retailer_array<'de, D>(deserializer: D) -> Result<Vec<RetailerName>, D::Error>
where
    D: Deserializer<'de>,
{
    let mut output: Vec<RetailerName> = Vec::new();

    let Ok(input_array) = serde_json::from_str::<Vec<String>>(&String::deserialize(deserializer)?)
    else {
        return Err(Error::custom("not valid JSON array"));
    };

    debug!("Parsing: {input_array:?}");

    for string_retailer in input_array {
        if let Ok(retailer) = RetailerName::from_str(&string_retailer) {
            output.push(retailer);
        } else {
            debug!("Invalid retailer mapping: {string_retailer:?}");
            return Err(Error::custom("invalid retailer"));
        }
    }

    Ok(output)
}

// responsible for turning a String input, into an optional number
fn string_to_cents<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: Deserializer<'de>,
{
    let input_string: Option<String> = Option::deserialize(deserializer)?;

    let Some(string_price) = input_string else {
        debug!("Invalid price: {:?}", input_string);
        return Err(Error::custom("invalid price"));
    };

    if string_price.is_empty() {
        return Ok(None);
    }

    let mut trimmed_price = string_price.clone();

    if trimmed_price.starts_with("$") {
        trimmed_price.remove(0);
    }

    trimmed_price = trimmed_price.replace(",", "");

    // lazily deal with missing cents
    // turns "100" -> "100.00"
    if !trimmed_price.contains(".") {
        trimmed_price += ".00";
    }

    let Some((dollars, cents)) = trimmed_price.split_once(".") else {
        debug!("Invalid format: {:?}", trimmed_price);
        return Err(Error::custom("invalid format"));
    };

    let parsed_dollars = match dollars.parse::<u32>() {
        Ok(dollar) => dollar,
        Err(_) => return Err(Error::custom("invalid dollar part")),
    };

    let parsed_cents = match cents.parse::<u32>() {
        Ok(cent) => cent,
        Err(_) => return Err(Error::custom("invalid cent part")),
    };

    let result = parsed_dollars * 100 + parsed_cents;

    debug!("Converted {} into {}", string_price, result);

    Ok(Some(result))
}
