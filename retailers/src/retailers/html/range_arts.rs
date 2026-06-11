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

const URL: &str = "https://rangearts.com/product-category/{category}/page/{page}/?stock_status=instock&per_page=24";

pub struct RangeArts {}

impl Default for RangeArts {
    fn default() -> Self {
        Self::new()
    }
}

impl RangeArts {
    pub fn new() -> Self {
        Self {}
    }
}

impl HtmlRetailerSuper for RangeArts {}

impl Retailer for RangeArts {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::RangeArts
    }
}

#[async_trait]
impl HtmlRetailer for RangeArts {
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

        let product_selector = Selector::parse("div.wd-product.instock").unwrap();

        let woocommerce = WooCommerceBuilder::default().build();

        let mut product_variants: Vec<String> = vec![];

        for element in fragment.select(&product_selector) {
            if extract_element_from_element(element, "span.price").is_err() {
                warn!("Skipping product that has no listed price");

                continue;
            }

            let add_cart_button = extract_element_from_element(element, "div.wd-add-btn > a")?;

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

            results.push(woocommerce.parse_product(
                element,
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
        let mut search: Vec<HtmlSearchQuery> = vec![
            HtmlSearchQuery {
                term: "firearms".into(),
                category: Category::Firearm,
            },
            HtmlSearchQuery {
                term: "ammo".into(),
                category: Category::Ammunition,
            },
        ];

        for other in ["optics", "care-cleaning", "accessories", "ppe", "gun-cases"] {
            search.push(HtmlSearchQuery {
                term: other.into(),
                category: Category::Other,
            });
        }

        search
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        WooCommerce::parse_max_pages(response)
    }
}
