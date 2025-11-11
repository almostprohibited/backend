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
            .with_product_url_selector("a.woocommerce-LoopProduct-link")
            .with_product_name_selector(
                "a.woocommerce-LoopProduct-link > h2.woocommerce-loop-product__title",
            )
            .with_image_url_selector("div.product-loop-thumbnail img")
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
