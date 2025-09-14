use async_trait::async_trait;
use common::result::{
    base::CrawlResult,
    enums::{Category, RetailerName},
    metadata::Metadata,
};
use metrics::{Metrics, put_metric};
use retailers::errors::RetailerError;

#[async_trait]
pub(crate) trait Client {
    async fn crawl(&mut self) -> Result<(), RetailerError>;

    fn get_results(&self) -> Vec<&CrawlResult>;

    fn get_retailer_name(&self) -> RetailerName;

    fn emit_metrics(&self) {
        for result in self.get_results() {
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

                if let Some(Metadata::Ammunition { .. }) = &result.metadata {
                    has_metadata = true;
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
}
