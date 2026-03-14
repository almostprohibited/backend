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
        generic_sitemap::get_search_queries,
    },
};

const URL: &str =
    "https://budgetshootersupply.ca/product-category/{category}/page/{page}/?per_page=24";
const SITEMAP_URL: &str = "https://budgetshootersupply.ca/product_cat-sitemap.xml";
const BASE_URL: &str = "https://budgetshootersupply.ca/product-category/";

pub struct BudgetShooterSupply {
    search_queries: Vec<HtmlSearchQuery>,
}

impl Default for BudgetShooterSupply {
    fn default() -> Self {
        Self::new()
    }
}

impl BudgetShooterSupply {
    pub fn new() -> Self {
        Self {
            search_queries: vec![],
        }
    }
}

impl HtmlRetailerSuper for BudgetShooterSupply {}

#[async_trait]
impl Retailer for BudgetShooterSupply {
    async fn init(&mut self) -> Result<(), RetailerError> {
        let queries = get_search_queries(SITEMAP_URL, BASE_URL, |link| {
            if link.starts_with("ammunition/") {
                return Some(HtmlSearchQuery {
                    term: link,
                    category: Category::Ammunition,
                });
            }

            // Some(HtmlSearchQuery {
            //     term: link,
            //     category: Category::Other,
            // })

            None
        })
        .await?;

        self.search_queries.extend(queries);

        Ok(())
    }

    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::BudgetShooterSupply
    }
}

#[async_trait]
impl HtmlRetailer for BudgetShooterSupply {
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

        let product_selector =
            Selector::parse("div.wd-products-element > div.products > div.wd-product.instock")
                .unwrap();

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
        self.search_queries.clone()
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        WooCommerce::parse_max_pages(response)
    }
}
