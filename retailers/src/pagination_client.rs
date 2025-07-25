use std::{collections::HashSet, time::Duration};

use common::result::{
    base::CrawlResult,
    enums::{Category, RetailerName},
};
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
    results: HashSet<CrawlResult>,
    pub total_bytes_tx: u64,
}

impl PaginationClient {
    pub fn new(retailer: Box<dyn Retailer + Send + Sync>) -> Self {
        Self {
            retailer,
            max_pages: 1,
            crawler: UnprotectedCrawler::new(),
            results: HashSet::new(),
            total_bytes_tx: 0,
        }
    }

    pub fn update_max_pages(&mut self, max_page: u64) {
        self.max_pages = max_page;
    }

    pub async fn crawl(&mut self) -> Result<(), RetailerError> {
        for term in self.retailer.get_search_terms() {
            if term.category == Category::Ammunition {
                continue;
            }

            self.paginate_calls(term).await?;
        }

        Ok(())
    }

    pub fn get_results(&self) -> Vec<&CrawlResult> {
        self.results.iter().collect()
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

            let inner_results = self.retailer.parse_response(&response, &term).await?;

            for inner in inner_results {
                let name = inner.name.clone();

                let insert_result = self.results.insert(inner);

                if !insert_result {
                    debug!("Failed to insert '{name}', hashed entry exists");
                } else {
                    debug!("Inserted '{name}'");
                }
            }

            current_page = current_page + 1;

            sleep(Duration::from_secs(CRAWL_COOLDOWN_SECS)).await;
        }

        Ok(())
    }

    async fn send_request(&mut self, request: Request) -> Result<String, RetailerError> {
        let response = self.crawler.make_web_request(request).await?;

        // nest these in `if let` statements instead of early exist since
        // thats more clean than several `return Ok()` statements
        if let Some(content_length_header) = response.headers.get("Content-Length") {
            if let Ok(content_length_str) = content_length_header.to_str() {
                if let Ok(content_length_u64) = content_length_str.parse::<u64>() {
                    self.total_bytes_tx += content_length_u64;
                }
            }
        };

        Ok(response.body)
    }
}
