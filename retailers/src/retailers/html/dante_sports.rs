use async_trait::async_trait;
use common::result::{
    base::CrawlResult,
    enums::{Category, RetailerName},
};
use crawler::request::{Request, RequestBuilder};
use scraper::{Html, Selector};
use tracing::{debug, warn};

use crate::{
    errors::RetailerError,
    structures::{HtmlRetailer, HtmlRetailerSuper, HtmlSearchQuery, Retailer},
    utils::{
        ecommerce::{WooCommerce, WooCommerceBuilder, WooCommerceNested},
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

const MAX_PER_PAGE: &str = "48";
const URL: &str = "https://www.dantesports.com/en/product-category/{category}/page/{page}/?per_page={max_per_page}&availability=in-stock";

pub struct DanteSports;

impl Default for DanteSports {
    fn default() -> Self {
        Self::new()
    }
}

impl DanteSports {
    pub fn new() -> Self {
        Self {}
    }
}

impl HtmlRetailerSuper for DanteSports {}

impl Retailer for DanteSports {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::DanteSports
    }
}

#[async_trait]
impl HtmlRetailer for DanteSports {
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

        let product_selector = Selector::parse("ul#products > li.product.instock > div").unwrap();

        let woocommerce_helper = WooCommerceBuilder::default()
            .with_image_url_selector("div.product-loop-thumbnail img")
            .build();

        let mut product_variants: Vec<String> = vec![];

        for product in html.select(&product_selector) {
            // hacky way to avoid the weird product they have for
            // special pricing
            if element_to_text(product).contains("PRICE ON REQUEST") {
                debug!("Skipping product that does not have public pricing");
                continue;
            }

            let add_cart_button = extract_element_from_element(product, "a.add_to_cart_button")?;

            if element_to_text(add_cart_button).to_lowercase() == "select options" {
                if let Ok(product_link) = element_extract_attr(add_cart_button, "href") {
                    product_variants.push(product_link);

                    continue;
                } else {
                    warn!(
                        "Failed to extract link for nested product, falling back to normal product"
                    );
                }
            }

            results.push(woocommerce_helper.parse_product(
                product,
                self.get_retailer_name(),
                search_term.category,
            )?);
        }

        for link in product_variants {
            results.extend(
                WooCommerce::parse_nested_products(
                    link,
                    search_term.category,
                    self.get_retailer_name(),
                )
                .await?,
            );
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
                term: "ammunition".into(),
                category: Category::Ammunition,
            },
        ]);

        let other_terms = ["riflescopes-optics", "accessories", "reloading", "storage"];

        for other in other_terms {
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
