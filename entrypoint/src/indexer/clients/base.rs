use async_trait::async_trait;
use common::result::{
    base::CrawlResult,
    enums::RetailerName,
    metadata::{Ammunition, Metadata},
};
use regex::Regex;
use retailers::errors::RetailerError;
use tracing::error;

#[async_trait]
pub(crate) trait Client {
    async fn crawl(&mut self) -> Result<(), RetailerError>;

    fn get_results(&self) -> Vec<&CrawlResult>;

    fn get_retailer_name(&self) -> RetailerName;
}

pub(crate) fn get_ammo_metadata(product_name: &String) -> Option<Metadata> {
    // TODO: I hate this, please find a different way of
    // parsing ammo counts other than constructing regex
    // every time this method is called
    let patterns = [
        Regex::new(r"(?i)(?:box|case|pack|tin) of (\d+)").expect("Ammo count regex to compile"),
        Regex::new(r"(?i)(\d+)\s*/?(?:ct|count|rd|rnd|round|pack|pc|shell|box)s?\b")
            .expect("Ammo count regex to compile"),
    ];

    for pattern in patterns {
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
