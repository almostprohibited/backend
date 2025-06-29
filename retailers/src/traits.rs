use std::time::Duration;

use async_trait::async_trait;
use common::result::{
    base::CrawlResult,
    enums::{Category, RetailerName},
};
use crawler::{request::Request, unprotected::UnprotectedCrawler};
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
    fn get_retailer_name(&self) -> RetailerName;
}

#[derive(Debug, Clone)]
pub struct SearchTerm {
    pub term: String,
    pub category: Category,
}
