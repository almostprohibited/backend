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
    structures::{HtmlRetailer, HtmlRetailerSuper, HtmlSearchQuery, Retailer},
    utils::{
        ecommerce::{WooCommerce, WooCommerceBuilder},
        html::extract_element_from_element,
    },
};

const MAX_PER_PAGE: &str = "48";
const URL: &str =
    "https://g4cgunstore.com/product-category/{category}/page/{page}/?per_page={max_per_page}";

pub struct G4CGunStore;

impl Default for G4CGunStore {
    fn default() -> Self {
        Self::new()
    }
}

impl G4CGunStore {
    pub fn new() -> Self {
        Self {}
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

impl HtmlRetailerSuper for G4CGunStore {}

impl Retailer for G4CGunStore {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::G4CGunStore
    }
}

#[async_trait]
impl HtmlRetailer for G4CGunStore {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &HtmlSearchQuery,
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
        search_term: &HtmlSearchQuery,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut results: Vec<CrawlResult> = Vec::new();

        let html = Html::parse_document(response);

        let product_selector =
            Selector::parse("div.products > div.product > div.product-wrapper").unwrap();

        let woocommerce_helper = WooCommerceBuilder::default().build();

        for product in html.select(&product_selector) {
            if !Self::is_in_stock(product) {
                // break instead of continue since products are in order
                // of in stock first, then all out of stock after
                break;
            }

            let result = woocommerce_helper.parse_product(
                product,
                self.get_retailer_name(),
                search_term.category,
            )?;

            results.push(result);
        }

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        let mut terms = Vec::from_iter([
            HtmlSearchQuery {
                term: "firearms".into(),
                category: Category::Firearm,
            },
            HtmlSearchQuery {
                term: "Ammunition".into(),
                category: Category::Ammunition,
            },
        ]);

        let other_terms = ["sights-optics", "accessories"];

        for other in other_terms {
            terms.push(HtmlSearchQuery {
                term: other.into(),
                category: Category::Other,
            });
        }

        terms
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        let html = Html::parse_document(response);
        let root_element = html.root_element();

        if Self::is_dead_page(root_element) {
            return Ok(0);
        }

        WooCommerce::parse_max_pages(response)
    }
}
