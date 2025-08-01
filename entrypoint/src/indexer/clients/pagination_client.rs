use std::{collections::HashSet, time::Duration};

use async_trait::async_trait;
use common::{
    result::{
        base::CrawlResult,
        enums::{Category, RetailerName},
    },
    utils::CRAWL_COOLDOWN_SECS,
};
use crawler::{request::Request, unprotected::UnprotectedCrawler};
use retailers::{
    errors::RetailerError,
    structures::{HtmlRetailerSuper, HtmlSearchQuery},
};
use tokio::time::sleep;
use tracing::{debug, trace};

use crate::clients::base::Client;

pub(crate) struct PaginationClient {
    retailer: Box<dyn HtmlRetailerSuper>,
    max_pages: u64,
    crawler: UnprotectedCrawler,
    results: HashSet<CrawlResult>,
}

#[async_trait]
impl Client for PaginationClient {
    async fn crawl(&mut self) -> Result<(), RetailerError> {
        for term in self.retailer.get_search_terms() {
            if term.category == Category::Ammunition {
                continue;
            }

            self.paginate_calls(term).await?;
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

impl PaginationClient {
    pub(crate) fn new(retailer: Box<dyn HtmlRetailerSuper>) -> Self {
        Self {
            retailer,
            max_pages: 1,
            crawler: UnprotectedCrawler::new(),
            results: HashSet::new(),
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
        Ok(self.crawler.make_web_request(request).await?.body)
    }
}
