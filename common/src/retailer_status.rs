use crate::serde_utils::bson_chrono_conversions;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::result::enums::RetailerName;

#[derive(Serialize, Default)]
pub struct RetailerProductCountsBuilder {
    firearms: u64,
    other: u64,
    ammo: u64,
    ammo_no_count: u64,
}

impl RetailerProductCountsBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_firearms(mut self, count: u64) -> Self {
        self.firearms = count;

        self
    }

    pub fn with_other(mut self, count: u64) -> Self {
        self.other = count;

        self
    }

    pub fn with_ammo(mut self, count: u64) -> Self {
        self.ammo = count;

        self
    }

    pub fn with_ammo_no_count(mut self, count: u64) -> Self {
        self.ammo_no_count = count;

        self
    }

    pub fn build(&self) -> RetailerProductCounts {
        RetailerProductCounts {
            firearms: self.firearms,
            other: self.other,
            ammo: self.ammo,
            ammo_no_count: self.ammo_no_count,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct RetailerProductCounts {
    pub firearms: u64,
    pub other: u64,
    pub ammo: u64,
    pub ammo_no_count: u64,
}

impl RetailerProductCounts {
    pub fn builder() -> RetailerProductCountsBuilder {
        RetailerProductCountsBuilder::new()
    }
}

#[derive(Serialize, Deserialize)]
pub struct RetailerStatus {
    pub retailer: RetailerName,
    pub counts: RetailerProductCounts,
    pub successful: bool,
    #[serde(with = "bson_chrono_conversions")]
    pub timestamp: DateTime<Utc>,
}
