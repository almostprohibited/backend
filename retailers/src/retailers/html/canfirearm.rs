use std::sync::LazyLock;

use async_trait::async_trait;
use common::result::{
    base::CrawlResult,
    enums::{Category, RetailerName},
};
use crawler::request::{Request, RequestBuilder};
use regex::Regex;
use scraper::{Html, Selector};
use tracing::{debug, warn};

use crate::{
    errors::RetailerError,
    structures::{HtmlRetailer, HtmlRetailerSuper, HtmlSearchQuery, Retailer},
    utils::{
        ecommerce::{WooCommerce, WooCommerceBuilder},
        html::{element_to_text, extract_element_from_element},
        regex::unwrap_regex_capture,
    },
};

static REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^BONUS WITH PURCHASE.*\s{2,}(.*)").expect("Regex to not change")
});

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

            // this is the first retailer I've seen that has an invalid product
            // image on their listings, and the actual image is a PDF of image
            // attachment in an email, just ignore products in this case
            match woocommerce.parse_product(element, self.get_retailer_name(), search_term.category)
            {
                Ok(mut product) => {
                    // remove the dumb "bonus with purchase" text they have
                    // if you want to discount the product, list it on sale
                    if let Ok(clean_name) = unwrap_regex_capture(&REGEX, &product.name) {
                        product.update_name(&clean_name);
                    }

                    results.push(product);
                }
                Err(err) => warn!("Invalid product parsed, ignoring: {err}"),
            };
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
