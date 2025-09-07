use std::{collections::HashMap, time::Duration};

use async_trait::async_trait;
use common::{
    result::{base::CrawlResult, enums::RetailerName},
    utils::CRAWL_COOLDOWN_SECS,
};
use crawler::unprotected::UnprotectedCrawler;
use retailers::{errors::RetailerError, structures::GqlRetailerSuper};
use tokio::time::sleep;
use tracing::debug;

use crate::clients::{
    base::Client,
    utils::{get_category_tier, get_key},
};

pub(crate) struct GqlClient {
    retailer: Box<dyn GqlRetailerSuper>,
    crawler: UnprotectedCrawler,
    results: HashMap<String, CrawlResult>,
}

impl GqlClient {
    pub(crate) fn new(retailer: Box<dyn GqlRetailerSuper>) -> Self {
        Self {
            retailer,
            crawler: UnprotectedCrawler::new(),
            results: HashMap::new(),
        }
    }

    // TODO: this method is repeated twice for each client, refactor this
    fn insert_result(&mut self, crawl_result: CrawlResult) {
        let key = get_key(&crawl_result);

        // deal with retailers that have the same product in multiple places
        if let Some(existing_result) = self.results.get_mut(&key)
            && get_category_tier(existing_result.category)
                < get_category_tier(crawl_result.category)
        {
            *existing_result = crawl_result;
        } else {
            self.results.insert(key, crawl_result);
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

            let response = self.crawler.make_web_request(request).await?;
            let response_body = response.body;

            pagination_token = self.retailer.get_pagination_token(&response_body)?;

            let results = self.retailer.parse_response(&response_body).await?;

            for crawled_result in results {
                self.insert_result(crawled_result);
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
