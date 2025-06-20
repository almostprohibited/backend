use std::time::Duration;

use async_trait::async_trait;
use common::result::{
    base::CrawlResult,
    enums::{Category, RetailerName},
};
use crawler::{request::Request, traits::Crawler, unprotected::UnprotectedCrawler};
use tokio::time::sleep;
use tracing::{debug, trace};

use crate::errors::RetailerError;

#[async_trait]
pub trait Retailer {
    // abstract methods
    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &SearchTerm,
    ) -> Result<Request, RetailerError>;
    async fn parse_response(
        &self,
        response: &String,
        search_term: &SearchTerm,
    ) -> Result<Vec<CrawlResult>, RetailerError>;
    fn get_search_terms(&self) -> Vec<SearchTerm>;
    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError>;
    fn get_crawler(&self) -> UnprotectedCrawler;
    fn get_page_cooldown(&self) -> u64;
    fn get_retailer_name(&self) -> RetailerName;

    // implemented methods
    async fn get_crawl_results(&self) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut results: Vec<CrawlResult> = Vec::new();

        for term in self.get_search_terms() {
            let mut page: u64 = 0;
            let mut max_page: u64 = 1;

            while page < max_page {
                let request = self.build_page_request(page, &term).await?;

                let result = self.send_request(self.get_crawler(), request).await?;

                trace!("{:?}", result);

                // commit a sin and attempt to change the loop conditions mid loop iteration
                if max_page == 1 {
                    let pages = self.get_num_pages(&result)?;
                    max_page = pages;

                    debug!("Changing max pages for '{:?}' to {}", term, max_page);
                }

                let mut interm_results = self.parse_response(&result, &term).await?;
                results.append(&mut interm_results);

                page = page + 1;

                sleep(Duration::from_secs(self.get_page_cooldown())).await;
            }

            sleep(Duration::from_secs(1)).await;
        }

        Ok(results)
    }

    async fn send_request(
        &self,
        crawler: UnprotectedCrawler,
        request: Request,
    ) -> Result<String, RetailerError> {
        Ok(crawler.make_web_request(request).await?)
    }
}

#[derive(Debug, Clone)]
pub struct SearchTerm {
    pub term: String,
    pub category: Category,
}
