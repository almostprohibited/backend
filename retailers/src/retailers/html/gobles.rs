use async_trait::async_trait;
use common::result::{
    base::CrawlResult,
    enums::{Category, RetailerName},
};
use crawler::request::{Request, RequestBuilder};
use tracing::debug;

use crate::{
    errors::RetailerError,
    structures::{HtmlRetailer, HtmlRetailerSuper, HtmlSearchQuery, Retailer},
    utils::ecommerce::LightSpeed,
};

const PAGE_LIMIT: u64 = 50;
const PRODUCT_BASE_URL: &str = "https://www.gobles.ca/";
const URL: &str =
    "https://www.gobles.ca/{category}/page{page}.html?limit={page_limit}&sort=default";

pub struct Gobles;

impl Default for Gobles {
    fn default() -> Self {
        Self::new()
    }
}

impl Gobles {
    pub fn new() -> Self {
        Self {}
    }
}

impl HtmlRetailerSuper for Gobles {}

impl Retailer for Gobles {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::Gobles
    }
}

#[async_trait]
impl HtmlRetailer for Gobles {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &HtmlSearchQuery,
    ) -> Result<Request, RetailerError> {
        let url = URL
            .replace("{category}", &search_term.term)
            .replace("{page_limit}", &PAGE_LIMIT.to_string())
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
        let mut products = LightSpeed::parse_products(
            PRODUCT_BASE_URL,
            "div.products-list > div",
            response,
            search_term,
            self.get_retailer_name(),
        )
        .await?;

        products.iter_mut().for_each(|product| {
            product.update_name(&product.name.replace(" - Available for Purchase", "").trim());
        });

        Ok(products)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        let mut search = vec![
            HtmlSearchQuery {
                term: "firearms".to_string(),
                category: Category::Firearm,
            },
            HtmlSearchQuery {
                term: "ammunition".to_string(),
                category: Category::Ammunition,
            },
        ];

        for term in [
            "optics",
            "accessories",
            "field-gear",
            "maintenance-storage",
            "reloading",
        ] {
            search.push(HtmlSearchQuery {
                term: term.to_string(),
                category: Category::Other,
            });
        }

        search
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        LightSpeed::get_max_pages(response, "div.pagination > ul > li:not(.active).number > a")
    }
}
