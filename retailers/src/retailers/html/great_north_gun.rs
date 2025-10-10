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
    utils::ecommerce::woocommerce::{WooCommerce, WooCommerceBuilder},
};

const URL: &str = "https://greatnorthgunco.ca/{category}/?product-page={page}";

pub struct GreatNorthGun;

impl GreatNorthGun {
    pub fn new() -> Self {
        Self {}
    }
}

impl HtmlRetailerSuper for GreatNorthGun {}

impl Retailer for GreatNorthGun {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::GreatNorthGun
    }
}

#[async_trait]
impl HtmlRetailer for GreatNorthGun {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &HtmlSearchQuery,
    ) -> Result<Request, RetailerError> {
        let url = URL
            .replace("{category}", &search_term.term)
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

        let product_selector =
            Selector::parse("div.woocommerce > ul.products > li.product").unwrap();

        let woocommerce_helper = WooCommerceBuilder::default()
            .with_product_url_selector("a.woocommerce-LoopProduct-link")
            .with_product_name_selector(
                "a.woocommerce-LoopProduct-link > h2.woocommerce-loop-product__title",
            )
            .with_image_url_selector("a.woocommerce-LoopProduct-link > img")
            .build();

        for product in html.select(&product_selector) {
            results.push(woocommerce_helper.parse_product(
                product,
                self.get_retailer_name(),
                search_term.category,
            )?);
        }

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        let mut terms: Vec<HtmlSearchQuery> = vec![];

        ["used", "new", "surplus"].iter().for_each(|term| {
            terms.push(HtmlSearchQuery {
                term: term.to_string(),
                category: Category::Firearm,
            })
        });

        ["accessories", "bayonets-and-knives"]
            .iter()
            .for_each(|term| {
                terms.push(HtmlSearchQuery {
                    term: term.to_string(),
                    category: Category::Other,
                })
            });

        terms
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        WooCommerce::parse_max_pages(response)
    }
}
