use std::time::Duration;

use common::result::{base::CrawlResult, enums::RetailerName};
use crawler::{request::Request, unprotected::UnprotectedCrawler};
use tokio::time::sleep;
use tracing::{debug, trace};

use crate::{
    errors::RetailerError,
    traits::{Retailer, SearchTerm},
};

pub const CRAWL_COOLDOWN_SECS: u64 = 10;

pub struct PaginationClient {
    retailer: Box<dyn Retailer + Send + Sync>,
    max_pages: u64,
    crawler: UnprotectedCrawler,
    results: Vec<CrawlResult>,
}

impl PaginationClient {
    pub fn new(retailer: Box<dyn Retailer + Send + Sync>) -> Self {
        Self {
            retailer,
            max_pages: 1,
            crawler: UnprotectedCrawler::new(),
            results: Vec::new(),
        }
    }

    pub fn update_max_pages(&mut self, max_page: u64) {
        self.max_pages = max_page;
    }

    pub async fn crawl(&mut self) -> Result<(), RetailerError> {
        for term in self.retailer.get_search_terms() {
            self.paginate_calls(term).await?;
        }

        Ok(())
    }

    pub fn get_results(&self) -> &Vec<CrawlResult> {
        &self.results
    }

    pub fn get_retailer_name(&self) -> RetailerName {
        self.retailer.get_retailer_name()
    }

    async fn paginate_calls(&mut self, term: SearchTerm) -> Result<(), RetailerError> {
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

            let mut inner_results = self.retailer.parse_response(&response, &term).await?;
            self.results.append(&mut inner_results);

            current_page = current_page + 1;

            sleep(Duration::from_secs(CRAWL_COOLDOWN_SECS)).await;
        }

        Ok(())
    }

    async fn send_request(&self, request: Request) -> Result<String, RetailerError> {
        Ok(self.crawler.make_web_request(request).await?.body)
    }
}
