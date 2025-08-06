use std::{collections::HashSet, time::Duration};

use async_trait::async_trait;
use common::{
    result::{
        base::CrawlResult,
        enums::{Category, RetailerName},
    },
    utils::CRAWL_COOLDOWN_SECS,
};
use crawler::unprotected::UnprotectedCrawler;
use retailers::{errors::RetailerError, structures::GqlRetailerSuper};
use tokio::time::sleep;
use tracing::debug;

use crate::clients::base::{Client, get_ammo_metadata};

pub(crate) struct GqlClient {
    retailer: Box<dyn GqlRetailerSuper>,
    crawler: UnprotectedCrawler,
    results: HashSet<CrawlResult>,
}

impl GqlClient {
    pub(crate) fn new(retailer: Box<dyn GqlRetailerSuper>) -> Self {
        Self {
            retailer,
            crawler: UnprotectedCrawler::new(),
            results: HashSet::new(),
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

            for mut crawled_result in results {
                if crawled_result.category == Category::Ammunition {
                    if let Some(metadata) = get_ammo_metadata(&crawled_result.name) {
                        crawled_result.set_metadata(metadata);
                    }
                }

                self.results.insert(crawled_result);
            }

            if pagination_token.is_none() {
                break;
            }

            sleep(Duration::from_secs(CRAWL_COOLDOWN_SECS)).await;
        }

        Ok(())
    }

    fn get_results(&self) -> Vec<&CrawlResult> {
        self.results.iter().collect()
    }

    fn get_retailer_name(&self) -> RetailerName {
        self.retailer.get_retailer_name()
    }
}
