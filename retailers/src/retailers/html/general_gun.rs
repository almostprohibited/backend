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

const URL: &str = "https://www.generalgun.com/product-category/{category}/page/{page}/?stock=instock&backorder=onbackorder";

pub struct GeneralGun {}

impl Default for GeneralGun {
    fn default() -> Self {
        Self::new()
    }
}

impl GeneralGun {
    pub fn new() -> Self {
        Self {}
    }
}

impl HtmlRetailerSuper for GeneralGun {}

impl Retailer for GeneralGun {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::GeneralGun
    }
}

#[async_trait]
impl HtmlRetailer for GeneralGun {
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

        let woocommerce = WooCommerceBuilder::default().build();

        for element in fragment.select(&product_selector) {
            results.push(woocommerce.parse_product(
                element,
                self.get_retailer_name(),
                search_term.category,
            )?);
        }

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        let mut search: Vec<HtmlSearchQuery> = vec![
            HtmlSearchQuery {
                term: "firearms".into(),
                category: Category::Firearm,
            },
            HtmlSearchQuery {
                term: "ammunition".into(),
                category: Category::Ammunition,
            },
        ];

        for other in [
            "optics",
            "parts-accessories",
            "personal-accessory",
            "reloading",
        ] {
            search.push(HtmlSearchQuery {
                term: other.into(),
                category: Category::Other,
            });
        }

        search
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        WooCommerce::parse_max_pages(response)
    }
}
