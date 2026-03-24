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
    utils::ecommerce::Magento,
};

const URL: &str = "https://x-reload.com/{category}.html?p={page}&product_list_limit=96&stock=1";

pub struct XReload;

impl Default for XReload {
    fn default() -> Self {
        Self::new()
    }
}

impl XReload {
    pub fn new() -> Self {
        Self {}
    }
}

impl HtmlRetailerSuper for XReload {}

impl Retailer for XReload {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::XReload
    }
}

#[async_trait]
impl HtmlRetailer for XReload {
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

        let product_selector = Selector::parse("ol.products > li.product").unwrap();

        for element in fragment.select(&product_selector) {
            match Magento::is_valid_product(element) {
                Ok(is_valid) => {
                    if !is_valid {
                        continue;
                    }
                }
                Err(err) => return Err(err),
            };

            results.push(Magento::parse_product(
                element,
                self.get_retailer_name(),
                search_term,
            )?);
        }

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        // unfortunately I don't have a way to filter out ammo specifically
        // otherwise, I would have just sent it with /products/

        let mut categories: Vec<HtmlSearchQuery> = vec![
            // they don't have firearms, but include anyways
            HtmlSearchQuery {
                term: "shooting/firearms".into(),
                category: Category::Firearm,
            },
            HtmlSearchQuery {
                term: "products/ammunition".into(),
                category: Category::Ammunition,
            },
        ];

        for category in [
            "products/reloading-components",
            "products/reloading",
            "products/shotshell-reloading",
            "shooting/cleaning-chemicals",
            "shooting/gun-parts-accessories",
            "shooting/shooting-acc",
            "shooting/firearm-safe",
            "shooting/tools",
            "optics",
        ] {
            categories.push(HtmlSearchQuery {
                term: category.into(),
                category: Category::Other,
            });
        }

        categories
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        Magento::get_pages(response)
    }
}
