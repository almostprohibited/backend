use std::{collections::HashMap, time::Duration};

use async_trait::async_trait;
use common::{
    constants::CRAWL_COOLDOWN_SECS,
    result::{base::CrawlResult, enums::RetailerName},
};
use crawler::unprotected::UnprotectedCrawler;
use retailers::{errors::RetailerError, structures::GqlRetailerSuper};
use tokio::time::sleep;
use tracing::debug;

use crate::clients::base::{Client, insert_result};

pub(crate) struct GqlClient {
    retailer: Box<dyn GqlRetailerSuper>,
    results: HashMap<String, CrawlResult>,
}

impl GqlClient {
    pub(crate) fn new(retailer: Box<dyn GqlRetailerSuper>) -> Self {
        Self {
            retailer,
            results: HashMap::new(),
        }
    }
}

#[async_trait]
impl Client for GqlClient {
    async fn crawl(&mut self) -> Result<(), RetailerError> {
        let mut pagination_token: Option<String> = None;

        loop {
            debug!("Using token: {pagination_token:?}");
            let request = self.retailer.build_page_request(pagination_token).await?;

            let response = UnprotectedCrawler::make_web_request(request).await?;
            let response_body = response.body;

            pagination_token = self.retailer.get_pagination_token(&response_body)?;

            let results = self.retailer.parse_response(&response_body).await?;

            for crawled_result in results {
                insert_result(&mut self.results, crawled_result);
            }

            if pagination_token.is_none() {
                break;
            }

            sleep(Duration::from_secs(CRAWL_COOLDOWN_SECS)).await;
        }

        Ok(())
    }

    fn get_results(&self) -> Vec<&CrawlResult> {
        self.results.values().collect()
    }

    fn get_retailer_name(&self) -> RetailerName {
        self.retailer.get_retailer_name()
    }
}
