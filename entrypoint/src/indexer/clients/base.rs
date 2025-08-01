use async_trait::async_trait;
use common::result::{base::CrawlResult, enums::RetailerName};
use retailers::errors::RetailerError;

#[async_trait]
pub(crate) trait Client {
    async fn crawl(&mut self) -> Result<(), RetailerError>;

    fn get_results(&self) -> Vec<&CrawlResult>;

    fn get_retailer_name(&self) -> RetailerName;
}
