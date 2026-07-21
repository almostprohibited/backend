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

const URL: &str = "https://swampdonkeyoutdoors.ca/product-category/{category}/page/{page}/";

const BLOCKED_TITLE_TERMS: [&str; 1] = ["pre-order"];

pub struct SwampDonkeyOutdoors {}

impl Default for SwampDonkeyOutdoors {
    fn default() -> Self {
        Self::new()
    }
}

impl SwampDonkeyOutdoors {
    pub fn new() -> Self {
        Self {}
    }
}

impl HtmlRetailerSuper for SwampDonkeyOutdoors {}

impl Retailer for SwampDonkeyOutdoors {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::SwampDonkeyOutdoors
    }
}

#[async_trait]
impl HtmlRetailer for SwampDonkeyOutdoors {
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

        let woocommerce_helper = WooCommerceBuilder::default().build();

        let html = Html::parse_document(response);
        let product_selector = Selector::parse("ul.products > li.product.instock").unwrap();

        for product in html.select(&product_selector) {
            let link_element =
                extract_element_from_element(product, "a.woocommerce-LoopProduct-link")?;
            let name = element_to_text(link_element).to_lowercase();

            if BLOCKED_TITLE_TERMS
                .iter()
                .any(|term| name.contains(&term.to_lowercase()))
            {
                debug!("Skipping {name} as it contains blocked term");
                continue;
            };

            if extract_element_from_element(product, "span.price").is_err() {
                debug!("Skipping {name} as it does not have listed price");
                continue;
            };

            let result = woocommerce_helper.parse_product(
                product,
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
                term: "firearm".into(),
                category: Category::Firearm,
            },
            HtmlSearchQuery {
                term: "ammo".into(),
                category: Category::Ammunition,
            },
        ]);

        let other_terms = [
            "optics",
            "reloading",
            "cleaning-maintenance",
            "blades",
            "parts-accessories",
        ];

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
