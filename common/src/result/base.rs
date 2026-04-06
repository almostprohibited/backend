use crate::serde_utils::from_datetime_to_u64_seconds;
use crate::serde_utils::from_u64_seconds_to_datetime;
use crate::utils::object_id_to_string;
use std::hash::{Hash, Hasher};
use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing::error;

use crate::result::metadata::Ammunition;
use crate::{
    result::{
        enums::{Category, RetailerName},
        metadata::Metadata,
    },
    utils::get_current_time,
};

static PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"(?i)(?:box|case|pack|tin|brick|can|pk|bottle|bulk) of (\d+)")
            .expect("Ammo count regex to compile"),
        Regex::new(
            r"(?i)(\d+)\s*/?(?:ct|count|rd|rnd|rs/box|round|pack|pc|shell|box|qty|pk|brick|per box)s?\b",
        )
        .expect("Ammo count regex to compile"),
    ]
});

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq)]
pub struct Price {
    pub regular_price: u64,
    pub sale_price: Option<u64>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct CrawlResult {
    #[serde(rename(deserialize = "_id"))]
    // TODO: this might break things if someone was to populate `id` accidentially
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(deserialize_with = "object_id_to_string")]
    pub id: Option<String>,
    pub name: String,
    pub url: String,
    pub price: Price,
    #[serde(serialize_with = "from_u64_seconds_to_datetime")]
    #[serde(deserialize_with = "from_datetime_to_u64_seconds")]
    pub query_time: u64,
    pub retailer: RetailerName,
    pub category: Category,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub metadata: Option<Metadata>,
}

// TNA forced my hand because they have so many products
// that are duplicated in their categories, now I need a hashing method
//
// I saw the same orange screwdriver set appear in 4 different categories
impl Hash for CrawlResult {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.url.hash(state);
        self.price.regular_price.hash(state);

        if let Some(sale_price) = self.price.sale_price {
            sale_price.hash(state);
        }
    }
}

impl PartialEq for CrawlResult {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.url == other.url && self.price == other.price
    }
}

impl Eq for CrawlResult {}

impl CrawlResult {
    pub fn new(
        name: String,
        url: String,
        price: Price,
        retailer: RetailerName,
        category: Category,
    ) -> Self {
        let time = get_current_time();

        // TODO: find a better way to fix the product pricing
        // in the case where both sale and price are the same
        let fixed_price = match price.regular_price == price.sale_price.unwrap_or_default() {
            true => Price {
                regular_price: price.regular_price,
                sale_price: None,
            },
            false => price,
        };

        let metadata = match category == Category::Ammunition {
            true => Self::get_ammo_metadata(&name),
            false => None,
        };

        Self {
            id: None,
            name,
            url,
            price: fixed_price,
            query_time: time,
            retailer,
            category,
            description: None,
            image_url: None,
            metadata,
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn with_image_url(mut self, image_url: String) -> Self {
        self.image_url = Some(image_url);
        self
    }

    pub fn with_metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn update_name(&mut self, name: &str) {
        self.name = name.to_string();

        let metadata = match self.category == Category::Ammunition {
            true => Self::get_ammo_metadata(&self.name),
            false => None,
        };

        self.metadata = metadata;
    }

    fn get_ammo_metadata(product_name: &String) -> Option<Metadata> {
        for pattern in PATTERNS.iter() {
            if let Some(capture) = pattern.captures(product_name) {
                let ammo_count = capture
                    .get(1)
                    .expect("Capture group should always match")
                    .as_str();

                let Ok(ammo_count_parsed) = ammo_count.parse() else {
                    error!(
                        "Failed to parse {ammo_count} into a u64 for {}, this shouldn't happen",
                        product_name
                    );

                    break;
                };

                return Some(Metadata::Ammunition(
                    Ammunition::new().with_round_count(ammo_count_parsed),
                ));
            }
        }

        None
    }
}
