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
        ecommerce::{WooCommerce, WooCommerceBuilder},
        html::extract_element_from_element,
    },
};

const URL: &str = "https://northernelitefirearms.ca/{category}/page/{page}/";

pub struct NorthernElite {}

impl Default for NorthernElite {
    fn default() -> Self {
        Self::new()
    }
}

impl NorthernElite {
    pub fn new() -> Self {
        Self {}
    }
}

impl HtmlRetailerSuper for NorthernElite {}

impl Retailer for NorthernElite {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::NorthernElite
    }
}

#[async_trait]
impl HtmlRetailer for NorthernElite {
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

        let woocommerce = WooCommerceBuilder::default()
            .with_product_name_selector("p.woocommerce-loop-product__title > a")
            .with_product_url_selector("p.woocommerce-loop-product__title > a")
            .with_image_url_selector("a > img")
            .build();

        for element in fragment.select(&product_selector) {
            if extract_element_from_element(element, "span.price").is_err() {
                warn!("Skipping product that has no listed price");

                continue;
            };

            let result = woocommerce.parse_product(
                element,
                self.get_retailer_name(),
                search_term.category,
            )?;

            results.push(result);
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

        for other in ["optics", "accessories"] {
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
