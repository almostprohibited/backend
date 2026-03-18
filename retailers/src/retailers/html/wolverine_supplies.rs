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
    utils::ecommerce::BigCommerce,
};

const URL: &str = "https://wolverinesupplies.com/{category}/?page={page}&in_stock=1";

pub struct WolverineSupplies;

impl Default for WolverineSupplies {
    fn default() -> Self {
        Self::new()
    }
}

impl WolverineSupplies {
    pub fn new() -> Self {
        Self {}
    }
}

impl HtmlRetailerSuper for WolverineSupplies {}

impl Retailer for WolverineSupplies {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::WolverineSupplies
    }
}

#[async_trait]
impl HtmlRetailer for WolverineSupplies {
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

        let product_selector = Selector::parse("ul.productGrid > li.product").unwrap();

        let bigcommerce_helper = BigCommerce::new();

        for element in fragment.select(&product_selector) {
            let result = bigcommerce_helper.parse_product(
                element,
                self.get_retailer_name(),
                search_term.category,
            )?;

            results.push(result);
        }

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        let mut terms: Vec<HtmlSearchQuery> = vec![
            HtmlSearchQuery {
                term: "ammunition".to_string(),
                category: Category::Ammunition,
            },
            HtmlSearchQuery {
                term: "firearms".to_string(), // this page contains barrels and odd bits and ends, should be sorted out in the other category
                category: Category::Firearm,
            },
        ];

        for other in [
            "FIREARMS-ACCESSORIES", // not a mistake, they have a category that is all uppercase
            "gearandkit",
            "optics",
            "parts",
            "reloading",
            "storagemaintenance",
        ] {
            terms.push(HtmlSearchQuery {
                term: other.to_string(),
                category: Category::Other,
            });
        }

        terms
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        BigCommerce::parse_max_pages(response)
    }
}
