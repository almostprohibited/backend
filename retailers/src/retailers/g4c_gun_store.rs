use async_trait::async_trait;
use common::result::{
    base::CrawlResult,
    enums::{Category, RetailerName},
};
use crawler::request::{Request, RequestBuilder};
use scraper::{ElementRef, Html, Selector};
use tracing::debug;

use crate::{
    errors::RetailerError,
    traits::{Retailer, SearchTerm},
    utils::{ecommerce::woocommerce::WooCommerce, html::extract_element_from_element},
};

const MAX_PER_PAGE: &str = "48";
const URL: &str =
    "https://g4cgunstore.com/product-category/{category}/page/{page}/?per_page={max_per_page}";

pub struct G4CGunStore {
    retailer: RetailerName,
}

impl G4CGunStore {
    pub fn new() -> Self {
        Self {
            retailer: RetailerName::G4CGunStore,
        }
    }

    fn is_in_stock(element: ElementRef) -> bool {
        return extract_element_from_element(element, "div.product-element-bottom > div.in-stock")
            .is_ok();
    }

    fn is_dead_page(element: ElementRef) -> bool {
        return extract_element_from_element(
            element,
            "div.product-element-bottom > div.out-of-stock",
        )
        .is_ok();
    }
}

#[async_trait]
impl Retailer for G4CGunStore {
    fn get_retailer_name(&self) -> RetailerName {
        self.retailer
    }

    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &SearchTerm,
    ) -> Result<Request, RetailerError> {
        let url = URL
            .replace("{category}", &search_term.term)
            .replace("{page}", &(page_num + 1).to_string())
            .replace("{max_per_page}", MAX_PER_PAGE);

        debug!("Setting page to {}", url);

        let request = RequestBuilder::new().set_url(url).build();

        Ok(request)
    }

    async fn parse_response(
        &self,
        response: &String,
        search_term: &SearchTerm,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut results: Vec<CrawlResult> = Vec::new();

        let html = Html::parse_document(&response);

        let product_selector =
            Selector::parse("div.products > div.product > div.product-wrapper").unwrap();

        for product in html.select(&product_selector) {
            if !Self::is_in_stock(product) {
                // break instead of continue since products are in order
                // of in stock first, then all out of stock after
                break;
            }

            let result = WooCommerce::parse_product(
                product,
                self.get_retailer_name(),
                search_term.category,
            )?;

            results.push(result);
        }

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<SearchTerm> {
        let mut terms = Vec::from_iter([SearchTerm {
            term: "firearms".into(),
            category: Category::Firearm,
        }]);

        let other_terms = ["sights-optics", "accessories"];

        for other in other_terms {
            terms.push(SearchTerm {
                term: other.into(),
                category: Category::Other,
            });
        }

        terms
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        let html = Html::parse_document(&response);
        let root_element = html.root_element();

        if Self::is_dead_page(root_element) {
            return Ok(0);
        }

        WooCommerce::parse_max_pages(response)
    }
}
