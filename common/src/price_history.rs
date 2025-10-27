use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiPriceHistoryInput {
    pub id: ObjectId,
}

#[derive(Serialize)]
pub struct ApiPriceHistoryOutput {
    pub history: Vec<PriceHistoryEntry>,
    pub max_price: PriceHistoryEntry,
    pub min_price: PriceHistoryEntry,
}

#[derive(Deserialize, Serialize)]
pub struct CollectionPriceHistory {
    pub name: String,
    pub url: String,
    pub price_history: Vec<PriceHistoryEntry>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct PriceHistoryEntry {
    pub regular_price: u64,
    pub sale_price: Option<u64>,
    pub query_time: u64,
}
