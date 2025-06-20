use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::result::{
    enums::{Category, RetailerName},
    metadata::Metadata,
};

#[derive(Deserialize, Serialize, Debug)]
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

impl CrawlResult {
    pub fn new(
        name: String,
        url: String,
        price: Price,
        retailer: RetailerName,
        category: Category,
    ) -> Self {
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap() // this should not fail since the current time is always > UNIX_EPOCH
            .as_secs();

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
}
