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

const PAGE_LIMIT: u64 = 100;
const SITE_MAP: &str = "https://www.solelyoutdoors.com/sitemap.xml";
const PRODUCT_BASE_URL: &str = "https://www.solelyoutdoors.com/";
const URL: &str =
    "https://www.solelyoutdoors.com/{category}/page{page}.html?limit={page_limit}&sort=default";

pub struct SoleyOutdoors {
    search_queries: Vec<HtmlSearchQuery>,
}

impl Default for SoleyOutdoors {
    fn default() -> Self {
        Self::new()
    }
}

impl SoleyOutdoors {
    pub fn new() -> Self {
        Self {
            search_queries: Vec::new(),
        }
    }
}

impl HtmlRetailerSuper for SoleyOutdoors {}

#[async_trait]
impl Retailer for SoleyOutdoors {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::SoleyOutdoors
    }

    async fn init(&mut self) -> Result<(), RetailerError> {
        let queries = get_search_queries(SITE_MAP, PRODUCT_BASE_URL, |link| {
            if link.contains("firearms/barrels/") {
                return None;
            }

            if link.starts_with("opitcs-plus/") // listen, soley is the one that misspelled optics here
                || link.starts_with("reloading/")
                || link.starts_with("shooting-firearm-acessories/")
            {
                return Some(HtmlSearchQuery {
                    term: link,
                    category: Category::Other,
                });
            } else if link.starts_with("ammunition/") {
                return Some(HtmlSearchQuery {
                    term: link,
                    category: Category::Ammunition,
                });
            } else if link.starts_with("firearms/") {
                return Some(HtmlSearchQuery {
                    term: link,
                    category: Category::Firearm,
                });
            };

            None
        })
        .await?;

        self.search_queries.extend(queries);

        Ok(())
    }
}

#[async_trait]
impl HtmlRetailer for SoleyOutdoors {
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
        LightSpeed::parse_products(
            PRODUCT_BASE_URL,
            "div.product-grid > div.product-block-holder",
            response,
            search_term,
            self.get_retailer_name(),
        )
        .await
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        self.search_queries.clone()
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        LightSpeed::get_max_pages(response, "div.paginate > ul > li:not(.active).number > a")
    }
}
