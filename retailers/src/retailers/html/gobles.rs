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
const SITE_MAP: &str = "https://www.gobles.ca/sitemap.xml";
const PRODUCT_BASE_URL: &str = "https://www.gobles.ca/";
const URL: &str =
    "https://www.gobles.ca/{category}/page{page}.html?limit={page_limit}&sort=default";

pub struct Gobles {
    search: Vec<HtmlSearchQuery>,
}

impl Default for Gobles {
    fn default() -> Self {
        Self::new()
    }
}

impl Gobles {
    pub fn new() -> Self {
        Self {
            search: Default::default(),
        }
    }
}

impl HtmlRetailerSuper for Gobles {}

#[async_trait]
impl Retailer for Gobles {
    async fn init(&mut self) -> Result<(), RetailerError> {
        let queries = get_search_queries(SITE_MAP, PRODUCT_BASE_URL, |link| {
            if link.starts_with("firearms/") {
                return Some(HtmlSearchQuery {
                    term: link,
                    category: Category::Firearm,
                });
            }

            if link == "ammunition" {
                return Some(HtmlSearchQuery {
                    term: link,
                    category: Category::Ammunition,
                });
            }

            if [
                "optics",
                "accessories",
                "field-gear",
                "maintenance-storage",
                "reloading",
            ]
            .contains(&link.as_str())
            {
                return Some(HtmlSearchQuery {
                    term: link,
                    category: Category::Other,
                });
            }

            None
        })
        .await?;

        self.search.extend(queries);

        debug!("{:?}", self.search);

        Ok(())
    }

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
        self.search.clone()
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        LightSpeed::get_max_pages(response, "div.pagination > ul > li:not(.active).number > a")
    }
}
