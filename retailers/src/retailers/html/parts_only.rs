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
        generic_sitemap::get_search_queries,
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

const BASE_URL: &str = "https://partsonly.ca/product-category/";
const SITEMAP_URL: &str = "https://partsonly.ca/wp-sitemap-taxonomies-product_cat-1.xml";
const URL: &str =
    "https://partsonly.ca/product-category/{category}/page/{page}/?filter_stock_status=instock";

pub struct PartsOnly {
    search_queries: Vec<HtmlSearchQuery>,
}

impl Default for PartsOnly {
    fn default() -> Self {
        Self::new()
    }
}

impl PartsOnly {
    pub fn new() -> Self {
        Self {
            search_queries: vec![],
        }
    }
}

impl HtmlRetailerSuper for PartsOnly {}

#[async_trait]
impl Retailer for PartsOnly {
    async fn init(&mut self) -> Result<(), RetailerError> {
        let queries = get_search_queries(SITEMAP_URL, BASE_URL, |link| {
            Some(HtmlSearchQuery {
                term: link,
                category: Category::Other,
            })
        })
        .await?;

        self.search_queries.extend(queries);

        Ok(())
    }

    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::PartsOnly
    }
}

#[async_trait]
impl HtmlRetailer for PartsOnly {
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

        let mut product_variants: Vec<String> = vec![];

        for element in fragment.select(&product_selector) {
            let Ok(price_element) = extract_element_from_element(element, "span.price") else {
                warn!("Skipping product that has no listed price");

                continue;
            };

            let add_cart_button = extract_element_from_element(element, "a.add_to_cart_button")?;

            if element_to_text(add_cart_button).to_lowercase() == "select options"
                && element_to_text(price_element).contains("–")
            {
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
        self.search_queries.clone()
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        WooCommerce::parse_max_pages(response)
    }
}
