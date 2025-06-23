use common::deserialize_disallow_empty_string::disallow_empty_string;
use common::result::enums::Category;
use mongodb::bson::Document;
use mongodb::bson::doc;
use serde::Deserialize;
use serde::Deserializer;
use serde::de::Error;
use serde_with::NoneAsEmptyString;
use serde_with::serde_as;
use strum_macros::EnumString;
use tracing::debug;
use tracing::trace;

use super::dedupe_stage::DedupeStage;
use super::match_stage::MatchStage;
use super::page_stage::PageStage;
use super::sort_stage::SortStage;

#[serde_as]
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct QueryParams {
    #[serde(deserialize_with = "disallow_empty_string")]
    pub(crate) query: String,
    #[serde_as(as = "NoneAsEmptyString")]
    #[serde(default)]
    pub(crate) page: Option<u32>,
    #[serde(deserialize_with = "string_to_cents")]
    #[serde(default)]
    pub(crate) min_price: Option<u32>,
    #[serde(deserialize_with = "string_to_cents")]
    #[serde(default)]
    pub(crate) max_price: Option<u32>,
    #[serde(default)]
    pub(crate) sort: Sort,
    #[serde(default)]
    pub(crate) category: Category,
}

#[derive(Debug, Default, Deserialize, EnumString, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub(crate) enum Sort {
    #[default]
    Relevant,
    PriceAsc,
    PriceDesc,
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

    if string_price == "" {
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
        trimmed_price = trimmed_price + ".00";
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

    return Ok(Some(result));
}

impl QueryParams {
    pub(crate) fn get_search_documents(&self) -> Vec<Document> {
        let mut documents: Vec<Document> = Vec::new();

        let flatten = doc! {
            "$replaceRoot": {
                "newRoot": "$doc"
            }
        };

        documents.append(
            &mut MatchStage::new(
                self.query.clone(),
                self.category,
                self.min_price,
                self.max_price,
            )
            .get_stage_documents(),
        );
        documents.append(&mut DedupeStage::new().get_stage_documents());
        documents.push(flatten);
        documents.append(&mut SortStage::new(self.sort).get_stage_documents());
        documents.append(&mut PageStage::new(self.page).get_stage_documents());

        trace!("Documents: {:#?}", documents);

        documents
    }

    pub(crate) fn get_count_documents(&self) -> Vec<Document> {
        let mut documents: Vec<Document> = Vec::new();

        let count = doc! {
            "$count": "total_count"
        };

        documents.append(
            &mut MatchStage::new(
                self.query.clone(),
                self.category,
                self.min_price,
                self.max_price,
            )
            .get_stage_documents(),
        );
        documents.append(&mut DedupeStage::new().get_stage_documents());
        documents.push(count);

        trace!("Documents: {:#?}", documents);

        documents
    }
}

pub(crate) trait StageDocument {
    fn get_stage_documents(&self) -> Vec<Document>;
}
