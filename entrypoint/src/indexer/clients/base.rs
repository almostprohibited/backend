use std::sync::LazyLock;

use async_trait::async_trait;
use common::result::{
    base::CrawlResult,
    enums::{Category, RetailerName},
    metadata::{Ammunition, Metadata},
};
use metrics::{Metrics, put_metric};
use regex::Regex;
use retailers::errors::RetailerError;
use tracing::error;

const PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"(?i)(?:box|case|pack|tin) of (\d+)").expect("Ammo count regex to compile"),
        Regex::new(r"(?i)(\d+)\s*/?(?:ct|count|rd|rnd|round|pack|pc|shell|box|qty)s?\b")
            .expect("Ammo count regex to compile"),
    ]
});

#[async_trait]
pub(crate) trait Client {
    async fn crawl(&mut self) -> Result<(), RetailerError>;

    fn get_results(&self) -> Vec<&CrawlResult>;

    fn get_retailer_name(&self) -> RetailerName;

    fn emit_metrics(&self, result: &CrawlResult) {
        let metric = match result.category {
            Category::Firearm => Some(Metrics::CrawledFirearm),
            Category::Ammunition => Some(Metrics::CrawledAmmunition),
            Category::Other => Some(Metrics::CrawledOther),
            _ => None,
        };

        if let Some(metric) = metric {
            put_metric!(metric, 1, "retailer" => self.get_retailer_name().to_string());
        }

        if result.category == Category::Ammunition {
            let mut has_metadata = false;

            if let Some(metadata) = &result.metadata {
                match metadata {
                    Metadata::Ammunition { .. } => {
                        has_metadata = true;
                    }
                    _ => {}
                }
            }

            if !has_metadata {
                put_metric!(
                    Metrics::CrawledAmmunitionNoRoundCount,
                    1,
                    "retailer" => self.get_retailer_name().to_string()
                );
            }
        }
    }
}

pub(crate) fn get_ammo_metadata(product_name: &String) -> Option<Metadata> {
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
