use async_trait::async_trait;
use common::result::{
    base::{CrawlResult, Price},
    enums::{Category, RetailerName},
};
use crawler::request::{Request, RequestBuilder};
use scraper::{Html, Selector};
use tracing::{debug, error};

use crate::{
    errors::RetailerError,
    traits::{HtmlRetailer, HtmlSearchQuery},
    utils::{
        conversions::{price_to_cents, string_to_u64},
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

const URL: &str = "aaa";

pub struct aaa {
    retailer: RetailerName,
}

impl aaa {
    pub fn new() -> Self {
        Self {
            retailer: RetailerName::aaa,
        }
    }
}

#[async_trait]
impl HtmlRetailer for aaa {
    fn get_retailer_name(&self) -> RetailerName {
        self.retailer
    }

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

        let html = Html::parse_document(&response);

        let product_selector = Selector::parse("aaa").unwrap();

        for product in html.select(&product_selector) {}

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        Vec::from_iter([HtmlSearchQuery {
            term: "a".into(),
            category: Category::Firearm,
        }])
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        let html = Html::parse_document(&response);

        Ok(0)
    }
}
