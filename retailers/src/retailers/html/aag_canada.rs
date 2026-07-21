use async_trait::async_trait;
use common::result::{
    base::CrawlResult,
    enums::{Category, RetailerName},
};
use crawler::request::{Request, RequestBuilder};
use tracing::debug;

use crate::{
    errors::RetailerError,
    structures::{HtmlRetailer, HtmlRetailerSuper, HtmlSearchQuery, Retailer},
    utils::ecommerce::{Shopify, shopify::PAGE_LIMIT},
};

const URL: &str =
    "https://aagcanada.ca/collections/{category}/products.json?limit={page_limit}&page={page}";
const BASE_URL: &str = "https://aagcanada.ca";
const DEFAULT_IMAGE: &str =
    "https://intersurplus.com/cdn/shopifycloud/storefront/assets/no-image-50-e6fb86f4_360x.gif";

pub struct AagCanada;

impl Default for AagCanada {
    fn default() -> Self {
        Self::new()
    }
}

impl AagCanada {
    pub fn new() -> Self {
        Self {}
    }
}

impl HtmlRetailerSuper for AagCanada {}

impl Retailer for AagCanada {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::AagCanada
    }
}

#[async_trait]
impl HtmlRetailer for AagCanada {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &HtmlSearchQuery,
    ) -> Result<Request, RetailerError> {
        let url = URL
            .replace("{page_limit}", &PAGE_LIMIT.to_string())
            .replace("{category}", &search_term.term.to_string())
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
        let helper = Shopify::new(DEFAULT_IMAGE, self.get_retailer_name(), BASE_URL);

        Ok(helper.parse_response(response, search_term)?)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        let mut terms = vec![HtmlSearchQuery {
            term: "firearms".into(),
            category: Category::Firearm,
        }];

        vec!["tactical-gear", "bayonet", "magazine-clips", "scope-sights"]
            .iter()
            .for_each(|term| {
                terms.push(HtmlSearchQuery {
                    term: term.to_string(),
                    category: Category::Other,
                });
            });

        terms
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        Shopify::get_pages(response)
    }
}
