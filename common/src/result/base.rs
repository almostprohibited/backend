use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

use crate::{
    result::{
        enums::{Category, RetailerName},
        metadata::Metadata,
    },
    utils::get_current_time,
};

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq)]
pub struct Price {
    pub regular_price: u64,
    pub sale_price: Option<u64>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct CrawlResult {
    pub name: String,
    pub url: String,
    pub price: Price,
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

        Self {
            name,
            url,
            price,
            query_time: time,
            retailer,
            category,
            description: None,
            image_url: None,
            metadata: None,
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

    pub fn set_metadata(&mut self, metadata: Metadata) {
        self.metadata = Some(metadata);
    }
}
