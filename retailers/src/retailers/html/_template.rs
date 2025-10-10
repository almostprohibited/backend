use async_trait::async_trait;
use common::result::{
    base::CrawlResult,
    enums::{Category, RetailerName},
};
use crawler::request::{Request, RequestBuilder};
use scraper::{Html, Selector};

use crate::{
    errors::RetailerError,
    structures::{HtmlRetailer, HtmlRetailerSuper, HtmlSearchQuery, Retailer},
};

const URL: &str = "aaa";

pub struct aaa;

impl Default for aaa {
    fn default() -> Self {
        Self::new()
    }
}

impl aaa {
    pub fn new() -> Self {
        Self {}
    }
}

impl HtmlRetailerSuper for aaa {}

impl Retailer for aaa {
    async fn new() -> Result<Self, RetailerError> {
        Ok(Self {})
    }

    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::aaa
    }
}

#[async_trait]
impl HtmlRetailer for aaa {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &HtmlSearchQuery,
    ) -> Result<Request, RetailerError> {
        let url = URL
            .replace("{category}", &search_term.term)
            .replace("{page}", &(page_num + 1).to_string());

        debug!("Setting page to {}", url);

        let request = RequestBuilder::new().set_url(url).build();

        Ok(request)
    }

    async fn parse_response(
        &self,
        response: &String,
        search_term: &HtmlSearchQuery,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut results: Vec<CrawlResult> = Vec::new();

        let html = Html::parse_document(response);

        let product_selector = Selector::parse("aaa").unwrap();

        for product in html.select(&product_selector) {}

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        let mut search_terms: Vec<HtmlSearchQuery> = Vec::new();

        [].iter().for_each(|category| {
            search_terms.push(HtmlSearchQuery {
                term: category.to_string(),
                category: Category::Firearm,
            })
        });

        [].iter().for_each(|category| {
            search_terms.push(HtmlSearchQuery {
                term: category.to_string(),
                category: Category::Ammunition,
            })
        });

        [].iter().for_each(|category| {
            search_terms.push(HtmlSearchQuery {
                term: category.to_string(),
                category: Category::Other,
            })
        });

        search_terms
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        Ok(0)
    }
}
