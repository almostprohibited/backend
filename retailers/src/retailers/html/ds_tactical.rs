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
        ecommerce::{BigCommerce, BigCommerceNested},
        html::{element_to_text, extract_element_from_element},
    },
};

const SITE_URL: &str = "https://dstactical.com/";
const URL: &str = "https://dstactical.com/{category}/?page={page}&in_stock=1";

pub struct DsTactical;

impl Default for DsTactical {
    fn default() -> Self {
        Self::new()
    }
}

impl DsTactical {
    pub fn new() -> Self {
        Self {}
    }
}

impl HtmlRetailerSuper for DsTactical {}

impl Retailer for DsTactical {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::DsTactical
    }
}

#[async_trait]
impl HtmlRetailer for DsTactical {
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
        let mut bigcommerce_helper = BigCommerce::new();
        let mut results: Vec<CrawlResult> = Vec::new();

        let fragment = Html::parse_document(response);

        let product_selector = Selector::parse("ul.productGrid > li.product").unwrap();

        for element in fragment.select(&product_selector) {
            let cart_button = extract_element_from_element(
                element,
                "div.card-figcaption--action-buttons > a[data-event-type='product-click']",
            )?;

            let cart_text = element_to_text(cart_button).to_lowercase();

            // ds tactical has products that have options
            // but they are all the same price
            // check if the price contain `-` which means diff price
            let price_element = extract_element_from_element(
                element,
                "div.price-section > span.price.price--withoutTax",
            )?;

            let price_text = element_to_text(price_element);

            if cart_text == "view options" && price_text.contains("-") {
                let _ = bigcommerce_helper
                    .enqueue_nested_product_element(element, search_term.category);

                continue;
            }

            let result = bigcommerce_helper.parse_product(
                element,
                self.get_retailer_name(),
                search_term.category,
            )?;

            results.push(result);
        }

        results.extend(
            bigcommerce_helper
                .parse_nested_products(SITE_URL, self.get_retailer_name())
                .await?,
        );

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        let mut terms = vec![];

        vec![
            "apparel",
            "bags-cases",
            "hard-gear",
            "soft-gear",
            "edged-tools-multi-tools",
            "lifestyle",
            "medical",
            "training-mil-sim-airsoft",
        ]
        .iter()
        .for_each(|term| {
            terms.push(HtmlSearchQuery {
                term: term.to_string(),
                category: Category::Other,
            });
        });

        terms
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        BigCommerce::parse_max_pages(response)
    }
}
