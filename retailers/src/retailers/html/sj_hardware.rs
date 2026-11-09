use async_trait::async_trait;
use common::result::{
    base::CrawlResult,
    enums::{Category, RetailerName},
};
use crawler::{
    WebClient,
    request::{Request, RequestBuilder},
};
use scraper::{Html, Selector};

use crate::{
    errors::RetailerError,
    structures::{HtmlRetailer, HtmlRetailerSuper, HtmlSearchQuery, Retailer},
    utils::{
        ecommerce::{BigCommerce, BigCommerceBuilder, BigCommerceNested},
        html::{element_to_text, extract_element_from_element},
    },
};

const SITE_URL: &str = "https://sjhardware.com/";
const URL: &str = "https://sjhardware.com/product-category/{category}/?page={page}";

pub struct SJHardware;

impl Default for SJHardware {
    fn default() -> Self {
        Self::new()
    }
}

impl SJHardware {
    pub fn new() -> Self {
        Self {}
    }
}

impl HtmlRetailerSuper for SJHardware {}

#[async_trait]
impl Retailer for SJHardware {
    async fn init(&mut self) -> Result<(), RetailerError> {
        // for some reason, they are now listing stuff in USD
        let request = RequestBuilder::new()
            .set_url(format!("{SITE_URL}?setCurrencyId=2"))
            .build();

        let _ = WebClient::make_web_request(request).await;

        Ok(())
    }

    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::SJHardware
    }
}

#[async_trait]
impl HtmlRetailer for SJHardware {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &HtmlSearchQuery,
    ) -> Result<Request, RetailerError> {
        let url = URL
            .replace("{category}", &search_term.term)
            .replace("{page}", &(page_num + 1).to_string());

        let request = RequestBuilder::new().set_url(url).build();

        Ok(request)
    }

    async fn parse_response(
        &self,
        response: &String,
        search_term: &HtmlSearchQuery,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let bigcommerce_helper = BigCommerceBuilder::default().build();

        let mut results: Vec<CrawlResult> = Vec::new();

        let html = Html::parse_document(response);
        let product_selector = Selector::parse("div.prod-grid > article.prod").unwrap();

        for product in html.select(&product_selector) {
            let cart_button = extract_element_from_element(product, "div.prod-actions > a.btn")?;
            let button_text = element_to_text(cart_button).to_lowercase();

            if button_text.contains("choose options") {
                // let _ = bigcommerce_helper
                //     .enqueue_nested_product_element(product, search_term.category);
            } else if button_text.contains("add to cart") {
                let result = bigcommerce_helper.parse_product(
                    product,
                    self.get_retailer_name(),
                    search_term.category,
                )?;

                results.push(result);
            }
        }

        results.extend(
            bigcommerce_helper
                .parse_nested_products(SITE_URL, self.get_retailer_name())
                .await?,
        );

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        let mut terms: Vec<HtmlSearchQuery> = vec![];

        ["firearms"].iter().for_each(|search| {
            terms.push(HtmlSearchQuery {
                term: search.to_string(),
                category: Category::Firearm,
            });
        });

        // https://sjhardware.com/product-category/ammo/
        // their ammo page says they don't ship ammo
        // oh well

        [
            "gun-cleaning",
            "tactical-gear",
            "medical-and-survival",
            "milsurp",
            "optics",
            "accessories",
            "precision-rifle-components",
            "reloading-components",
        ]
        .iter()
        .for_each(|search| {
            terms.push(HtmlSearchQuery {
                term: search.to_string(),
                category: Category::Other,
            });
        });

        terms
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        BigCommerce::parse_max_pages(response)
    }
}
