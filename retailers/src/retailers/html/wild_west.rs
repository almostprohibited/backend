use async_trait::async_trait;
use common::result::{
    base::CrawlResult,
    enums::{Category, RetailerName},
};
use crawler::request::{Request, RequestBuilder};
use scraper::{Html, Selector};
use tracing::debug;

use crate::{
    errors::RetailerError,
    structures::{HtmlRetailer, HtmlRetailerSuper, HtmlSearchQuery, Retailer},
    utils::ecommerce::{WooCommerce, WooCommerceBuilder},
};

const URL: &str = "https://www.gun-shop.ca/product-category/{category}/page/{page}/?in_stock=1";

pub struct WildWest {}

impl Default for WildWest {
    fn default() -> Self {
        Self::new()
    }
}

impl WildWest {
    pub fn new() -> Self {
        Self {}
    }
}

impl HtmlRetailerSuper for WildWest {}

impl Retailer for WildWest {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::WildWest
    }
}

#[async_trait]
impl HtmlRetailer for WildWest {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &HtmlSearchQuery,
    ) -> Result<Request, RetailerError> {
        let url = URL
            .replace("{category}", &search_term.term)
            .replace("{page}", (page_num + 1).to_string().as_str());

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

        let fragment = Html::parse_document(response);

        let product_selector = Selector::parse("ul.products > li.product.instock").unwrap();

        let woocommerce = WooCommerceBuilder::default().build();

        for element in fragment.select(&product_selector) {
            let result = woocommerce.parse_product(
                element,
                self.get_retailer_name(),
                search_term.category,
            )?;

            results.push(result);
        }

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        // NOTE: /competition-shotgun/ category seems to be covered by the others

        let mut terms = Vec::from_iter([
            HtmlSearchQuery {
                term: "firearms-2".into(),
                category: Category::Firearm,
            },
            HtmlSearchQuery {
                term: "ammunition-2".into(),
                category: Category::Ammunition,
            },
        ]);

        for other in [
            "firearm-parts-accessories",
            "outdoor-field-gear",
            "everyday-carry-edc",
            "cleaning-maintenance",
            "apparel-merch",
        ] {
            terms.push(HtmlSearchQuery {
                term: other.into(),
                category: Category::Other,
            });
        }

        terms
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        WooCommerce::parse_max_pages(response)
    }
}
