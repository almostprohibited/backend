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
    utils::{
        ecommerce::{WooCommerce, WooCommerceBuilder},
        html::{element_to_text, extract_element_from_element},
    },
};

const URL: &str =
    "https://canfirearm.com/product-category/{category}/page/{page}/?stock_status=instock";

pub struct CanFirearm;

impl Default for CanFirearm {
    fn default() -> Self {
        Self::new()
    }
}

impl CanFirearm {
    pub fn new() -> Self {
        Self {}
    }
}

impl HtmlRetailerSuper for CanFirearm {}

impl Retailer for CanFirearm {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::CanFirearm
    }
}

#[async_trait]
impl HtmlRetailer for CanFirearm {
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

        let product_selector = Selector::parse("div.products > div.product.instock").unwrap();

        let woocommerce = WooCommerceBuilder::default()
            .with_product_url_selector("h3.wd-entities-title > a")
            .with_product_name_selector("h3.wd-entities-title > a")
            .build();

        for element in fragment.select(&product_selector) {
            let Ok(add_cart_button) = extract_element_from_element(
                element,
                "div.wd-bottom-actions a.button.add_to_cart_button",
            ) else {
                debug!("Skipping product, no add cart button");
                continue;
            };

            if element_to_text(add_cart_button).to_ascii_lowercase() != "add to cart".to_string() {
                debug!("Skipping product, button found but does not say cart");
                continue;
            }

            results.push(woocommerce.parse_product(
                element,
                self.get_retailer_name(),
                search_term.category,
            )?);
        }

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        let mut terms: Vec<HtmlSearchQuery> = vec![
            HtmlSearchQuery {
                term: "ammunition".to_string(),
                category: Category::Ammunition,
            },
            HtmlSearchQuery {
                term: "firearms".to_string(),
                category: Category::Firearm,
            },
        ];

        for other in ["accessories", "tactical-gear"] {
            terms.push(HtmlSearchQuery {
                term: other.to_string(),
                category: Category::Other,
            });
        }

        terms
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        WooCommerce::parse_max_pages(response)
    }
}
