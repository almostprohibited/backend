use std::u64::MAX;

use async_trait::async_trait;
use common::result::{
    base::CrawlResult,
    enums::{Category, RetailerName},
};
use crawler::request::{Request, RequestBuilder};
use scraper::{Html, Selector};
use serde::Deserialize;
use tracing::debug;

use crate::{
    errors::RetailerError,
    structures::{HtmlRetailer, HtmlRetailerSuper, HtmlSearchQuery, Retailer},
    utils::{
        ecommerce::{woocommerce::WooCommerceBuilder, woocommerce_nested::WooCommerceNested},
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiResponse {
    items: String,
    next_page: String,
}

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

    fn parse_clean_html(&self, raw_html: String) -> Html {
        Html::parse_fragment(&raw_html.replace("\t", "").replace("\n", ""))
    }

    fn is_ending_page(&self, parsed_response: ApiResponse) -> Result<bool, RetailerError> {
        if parsed_response.next_page.is_empty() {
            return Ok(true);
        }

        let html = self.parse_clean_html(parsed_response.items);
        let selector = Selector::parse("div:is(.onbackorder, .outofstock)").unwrap();

        Ok(html.select(&selector).next().is_some())
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

        let parsed_response = serde_json::from_str::<ApiResponse>(response)?;

        let products = {
            let html = Html::parse_document(&parsed_response.items);
            let product_selector = Selector::parse("div.instock > div.product-wrapper").unwrap();
            html.select(&product_selector)
                .map(|element| element.html().clone())
                .collect::<Vec<_>>()
        };

        let css_selector = "div.wd-product-header > h3 > a";

        let woocommerce_helper = WooCommerceBuilder::default()
            .with_product_name_selector(css_selector)
            .with_product_url_selector(css_selector)
            .build();
        let mut woocommerce_nested = WooCommerceNested::new(self.get_retailer_name());

        for raw_html in products {
            let parsed_html = Html::parse_fragment(&raw_html);
            let product = parsed_html.root_element();

            let add_cart_button =
                extract_element_from_element(product, "div.wd-product-footer > div > a")?;

            if element_to_text(add_cart_button).to_lowercase() == "select options" {
                let product_url_element = extract_element_from_element(product, css_selector)?;
                let product_url = element_extract_attr(product_url_element, "href")?;

                woocommerce_nested.enqueue_product(product_url, search_term.category);

                continue;
            }

            results.push(woocommerce_helper.parse_product(
                product,
                self.get_retailer_name(),
                search_term.category,
            )?);
        }

        results.extend(woocommerce_nested.parse_nested().await?);

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
        let parsed_response = serde_json::from_str::<ApiResponse>(response)?;

        if parsed_response.next_page.is_empty() || self.is_ending_page(parsed_response)? {
            return Ok(0);
        }

        Ok(MAX)
    }
}
