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
    utils::{ecommerce::LightSpeed, generic_sitemap::get_search_queries},
};

const PAGE_LIMIT: u64 = 50;
const PRODUCT_BASE_URL: &str = "https://www.easthilloutdoors.com/";
const SITE_MAP: &str = "https://www.easthilloutdoors.com/sitemap.xml";
const URL: &str = "https://www.easthilloutdoors.com/{category}/page{page}.html/?limit={page_limit}";

pub struct EasthillOutdoors {
    search_queries: Vec<HtmlSearchQuery>,
}

impl Default for EasthillOutdoors {
    fn default() -> Self {
        Self::new()
    }
}

impl EasthillOutdoors {
    pub fn new() -> Self {
        Self {
            search_queries: Vec::new(),
        }
    }
}

impl HtmlRetailerSuper for EasthillOutdoors {}

#[async_trait]
impl Retailer for EasthillOutdoors {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::EasthillOutdoors
    }

    async fn init(&mut self) -> Result<(), RetailerError> {
        let queries = get_search_queries(SITE_MAP, PRODUCT_BASE_URL, |link| {
            if link.starts_with("firearms/") {
                if link.contains("air-rifles") {
                    return None;
                }

                if [
                    "used-firearms/",
                    "rifle-centerfire/",
                    "rifle-rimfire/",
                    "shotguns/",
                    // "pistols", // one day
                    "muzzleloaders",
                ]
                .iter()
                .any(|param| link.contains(param))
                {
                    return Some(HtmlSearchQuery {
                        term: link,
                        category: Category::Firearm,
                    });
                }

                return Some(HtmlSearchQuery {
                    term: link,
                    category: Category::Other,
                });
            }

            if link.starts_with("ammo/") {
                if link.contains("reloading") {
                    return Some(HtmlSearchQuery {
                        term: link,
                        category: Category::Other,
                    });
                }

                // some muzzleloading stuff that is technically
                // "other" will be caught in here, problem for
                // later me to fix
                return Some(HtmlSearchQuery {
                    term: link,
                    category: Category::Ammunition,
                });
            }

            None
        })
        .await?;

        self.search_queries.extend(queries);

        debug!("{:?}", self.search_queries);

        Ok(())
    }
}

#[async_trait]
impl HtmlRetailer for EasthillOutdoors {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &HtmlSearchQuery,
    ) -> Result<Request, RetailerError> {
        let url = URL
            .replace("{category}", &search_term.term)
            .replace("{page}", (page_num + 1).to_string().as_str())
            .replace("{page_limit}", &PAGE_LIMIT.to_string());

        debug!("Setting page to {}", url);

        let request = RequestBuilder::new().set_url(url).build();

        Ok(request)
    }

    async fn parse_response(
        &self,
        response: &String,
        search_term: &HtmlSearchQuery,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let products = LightSpeed::parse_products(
            PRODUCT_BASE_URL,
            "div.product.hasstock",
            response,
            search_term,
            self.get_retailer_name(),
        )
        .await?;

        Ok(products)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        self.search_queries.clone()
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        LightSpeed::get_max_pages(response, "div.pager > ul > li:not(.active).number > a")
    }
}
