use std::{collections::HashMap, time::Duration};

use async_trait::async_trait;
use common::{
    constants::CRAWL_COOLDOWN_SECS,
    result::{base::CrawlResult, enums::RetailerName},
};
use crawler::{request::Request, unprotected::UnprotectedCrawler};
use retailers::{
    errors::RetailerError,
    structures::{HtmlRetailerSuper, HtmlSearchQuery},
};
use tokio::time::sleep;
use tracing::{debug, trace};

use crate::clients::{
    base::Client,
    utils::{get_category_tier, get_key},
};

pub(crate) struct PaginationClient {
    retailer: Box<dyn HtmlRetailerSuper>,
    max_pages: u64,
    crawler: UnprotectedCrawler,
    results: HashMap<String, CrawlResult>,
}

#[async_trait]
impl Client for PaginationClient {
    async fn crawl(&mut self) -> Result<(), RetailerError> {
        for term in self.retailer.get_search_terms() {
            self.paginate_calls(term).await?;
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

impl PaginationClient {
    pub(crate) fn new(retailer: Box<dyn HtmlRetailerSuper>) -> Self {
        Self {
            retailer,
            max_pages: 1,
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

    pub(crate) fn update_max_pages(&mut self, max_page: u64) {
        self.max_pages = max_page;
    }

    async fn paginate_calls(&mut self, term: HtmlSearchQuery) -> Result<(), RetailerError> {
        self.update_max_pages(1);
        let mut current_page: u64 = 0;

        while current_page < self.max_pages {
            let request = self
                .retailer
                .build_page_request(current_page, &term)
                .await?;

            let response = self.send_request(request).await?;
            trace!("{response:?}");

            // commit a sin and attempt to change the loop conditions mid loop iteration
            self.update_max_pages(self.retailer.get_num_pages(&response)?);
            debug!("Changing max pages to {}", self.max_pages);

            let results = self.retailer.parse_response(&response, &term).await?;

            for crawled_result in results {
                self.insert_result(crawled_result);
            }

            current_page += 1;

            sleep(Duration::from_secs(CRAWL_COOLDOWN_SECS)).await;
        }

        Ok(())
    }

    async fn send_request(&mut self, request: Request) -> Result<String, RetailerError> {
        Ok(self.crawler.make_web_request(request).await?.body)
    }
}
