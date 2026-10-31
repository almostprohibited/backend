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
        ecommerce::{WooCommerce, WooCommerceBuilder},
        html::{element_to_text, extract_element_from_element},
    },
};

const URL: &str = "https://tigerarms.ca/product-category/{category}/page/{page}/?instock_filter=1";

pub struct TigerArms {}

impl Default for TigerArms {
    fn default() -> Self {
        Self::new()
    }
}

impl TigerArms {
    pub fn new() -> Self {
        Self {}
    }
}

impl HtmlRetailerSuper for TigerArms {}

impl Retailer for TigerArms {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::TigerArms
    }
}

#[async_trait]
impl HtmlRetailer for TigerArms {
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

        // why would you do this? you have the HTML inside of a JS script
        // which just ends up appending the HTML to the DOM
        let script_element =
            extract_element_from_element(fragment.root_element(), "ul.products > script")?;
        let script_contents = serde_json::from_str::<String>(&element_to_text(script_element))?;

        let products = Html::parse_fragment(&script_contents);

        let product_selector = Selector::parse("li.product.instock").unwrap();

        let woocommerce = WooCommerceBuilder::default()
            .with_image_url_selector("div.inner > img")
            .build();

        for element in products.select(&product_selector) {
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

        for other in [
            "shooting-accessories",
            "optics",
            "mounts-bases",
            "firearm-accessories",
            "reloading-supplies",
            "cleaning-supplies",
            "flashlights-headlamps",
            "knives-axes-tools",
        ] {
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
