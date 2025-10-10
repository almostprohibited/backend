use async_trait::async_trait;
use common::result::{
    base::CrawlResult,
    enums::{Category, RetailerName},
};
use crawler::request::Request;

use crate::errors::RetailerError;

pub trait HtmlRetailerSuper: HtmlRetailer + Retailer + Send + Sync {}
pub trait GqlRetailerSuper: GqlRetailer + Retailer + Send + Sync {}

#[async_trait]
pub trait Retailer {
    fn get_retailer_name(&self) -> RetailerName;

    async fn init(&mut self) -> Result<(), RetailerError> {
        Ok(())
    }
}

#[async_trait]
pub trait HtmlRetailer {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &HtmlSearchQuery,
    ) -> Result<Request, RetailerError>;

    async fn parse_response(
        &self,
        response: &String,
        search_term: &HtmlSearchQuery,
    ) -> Result<Vec<CrawlResult>, RetailerError>;

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery>;

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError>;
}

#[async_trait]
pub trait GqlRetailer {
    async fn build_page_request(
        &self,
        pagination_token: Option<String>,
    ) -> Result<Request, RetailerError>;

    async fn parse_response(&self, response: &str) -> Result<Vec<CrawlResult>, RetailerError>;

    fn get_pagination_token(&self, response: &str) -> Result<Option<String>, RetailerError>;
}

#[derive(Debug, Clone)]
pub struct HtmlSearchQuery {
    pub term: String,
    pub category: Category,
}
