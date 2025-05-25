use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::constants::{ActionType, AmmunitionType, FirearmClass, FirearmType, RetailerName};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct FirearmPrice {
    pub regular_price: u64,
    pub sale_price: Option<u64>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct FirearmResult {
    pub name: String,
    pub link: String,
    pub price: FirearmPrice,
    pub query_time: u64,
    pub retailer: RetailerName,
    pub description: Option<String>,
    pub thumbnail_link: Option<String>,
    pub action_type: Option<ActionType>,
    pub ammo_type: Option<AmmunitionType>,
    pub firearm_class: Option<FirearmClass>,
    pub firearm_type: Option<FirearmType>,
}

impl FirearmResult {
    pub fn new(
        name: impl Into<String>,
        link: impl Into<String>,
        price: FirearmPrice,
        retailer: RetailerName,
    ) -> Self {
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap() // this should not fail since the current time is always > UNIX_EPOCH
            .as_secs();

        Self {
            name: name.into(),
            link: link.into(),
            price,
            query_time: time,
            retailer,
            ..Default::default()
        }
    }
}
