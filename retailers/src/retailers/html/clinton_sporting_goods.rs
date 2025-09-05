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

const URL: &str = "https://clintonsporting.com/product-category/{category}/page/{page}/";

pub struct ClintonSportingGoods;

impl ClintonSportingGoods {
    pub fn new() -> Self {
        Self {}
    }
}

impl HtmlRetailerSuper for ClintonSportingGoods {}

impl Retailer for ClintonSportingGoods {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::ClintonSportingGoods
    }
}

#[async_trait]
impl HtmlRetailer for ClintonSportingGoods {
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

        let html = Html::parse_document(&response);

        let product_selector = Selector::parse("ul.products > li.product").unwrap();

        let woocommerce_helper = WooCommerceBuilder::default()
            .with_product_url_selector("a.woocommerce-LoopProduct-link")
            .with_product_name_selector(
                "a.woocommerce-LoopProduct-link > h2.woocommerce-loop-product__title",
            )
            .with_image_url_selector("a.woocommerce-LoopProduct-link > picture > img")
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
        // the firearm category contains non firearm stuff, like
        // a wader and a poncho
        // if people don't like it, they can complain to the store
        let mut query = vec![
            HtmlSearchQuery {
                term: "firearms".into(),
                category: Category::Firearm,
            },
            HtmlSearchQuery {
                term: "ammunition".into(),
                category: Category::Ammunition,
            },
        ];

        [
            "accessories/attachments",
            "accessories/bases",
            "accessories/muzzle-loading",
            "accessories/accessories_rings",
            "accessories/gun-cases",
            "accessories/slings",
            "accessories/security",
            "accessories/magazines-clips",
            "accessories/choke-tubes",
            "accessories/misc-accessories", // I don't know whats in here, good luck
        ]
        .iter()
        .for_each(|term| {
            query.push(HtmlSearchQuery {
                term: term.to_string(),
                category: Category::Other,
            })
        });

        query
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        WooCommerce::parse_max_pages(response)
    }
}
