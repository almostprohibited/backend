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
        ecommerce::{WooCommerce, WooCommerceBuilder, WooCommerceNested},
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

const MAX_PER_PAGE: &str = "45";
const URL: &str = "https://victoryridgesports.ca/product-category/{category}/page/{page}/?woo_ajax=1&per_page={max_per_page}";

pub struct VictoryRidgeSports;

impl Default for VictoryRidgeSports {
    fn default() -> Self {
        Self::new()
    }
}

impl VictoryRidgeSports {
    pub fn new() -> Self {
        Self {}
    }
}

impl HtmlRetailerSuper for VictoryRidgeSports {}

impl Retailer for VictoryRidgeSports {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::VictoryRidgeSports
    }
}

#[async_trait]
impl HtmlRetailer for VictoryRidgeSports {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &HtmlSearchQuery,
    ) -> Result<Request, RetailerError> {
        let url = URL
            .replace("{category}", &search_term.term)
            .replace("{max_per_page}", MAX_PER_PAGE)
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

        let html = Html::parse_document(response);
        let product_selector = Selector::parse("ul.products > li.product.instock").unwrap();

        let css_selector = "a.ast-loop-product__link";

        let woocommerce_helper = WooCommerceBuilder::default()
            .with_product_name_selector(css_selector)
            .with_product_url_selector(css_selector)
            .build();

        let mut variant_links: Vec<String> = vec![];

        for product in html.select(&product_selector) {
            let add_cart_button =
                extract_element_from_element(product, "a.add_to_cart_button.button")?;

            match element_to_text(add_cart_button).to_lowercase().as_str() {
                "select options" => {
                    let product_url = element_extract_attr(add_cart_button, "href")?;

                    variant_links.push(product_url);
                }
                "add to cart" => {
                    results.push(woocommerce_helper.parse_product(
                        product,
                        self.get_retailer_name(),
                        search_term.category,
                    )?);
                }
                _ => {
                    debug!("Skipping product with no cart button")
                }
            };
        }

        for link in variant_links {
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
        let mut search_terms: Vec<HtmlSearchQuery> = Vec::new();

        // they have a /rifles/ URI, but it seems to be incomplete at first glance?
        [
            "shooting/rifles",
            "shooting/pre-owned/pre-owned-rifles",
            "shooting/pre-owned/pre-owned-shotguns",
            "shooting/shotguns",
            // "shooting/rimfire" // there's ammo in here, and they don't seem to have rimfires
        ]
        .iter()
        .for_each(|category| {
            search_terms.push(HtmlSearchQuery {
                term: category.to_string(),
                category: Category::Firearm,
            })
        });

        ["shooting/ammunition"].iter().for_each(|category| {
            search_terms.push(HtmlSearchQuery {
                term: category.to_string(),
                category: Category::Ammunition,
            })
        });

        [
            "shooting/training",
            "shooting/maple-ridge-armoury-parts",
            "shooting/shooting-equipment",
            "shooting/parts-accessories",
            "shooting/reloading",
            "optics",
        ]
        .iter()
        .for_each(|category| {
            search_terms.push(HtmlSearchQuery {
                term: category.to_string(),
                category: Category::Other,
            })
        });

        search_terms
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        let html = Html::parse_fragment(response);
        let selector = Selector::parse("li:is(.onbackorder, .outofstock)").unwrap();

        if html.select(&selector).next().is_some() {
            return Ok(0);
        }

        WooCommerce::parse_max_pages(response)
    }
}
