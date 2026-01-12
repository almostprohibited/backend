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
    utils::{ecommerce::BigCommerce, generic_sitemap::get_search_queries},
};

const PRODUCT_BASE_URL: &str = "https://frontierfirearms.ca/";
const SITE_MAP: &str = "https://frontierfirearms.ca/xmlsitemap.php?type=categories&page=1";
const URL: &str = "https://frontierfirearms.ca/{category}?in_stock=1&page={page}";

pub struct FrontierFirearms {
    search_queries: Vec<HtmlSearchQuery>,
}

impl Default for FrontierFirearms {
    fn default() -> Self {
        Self::new()
    }
}

impl FrontierFirearms {
    pub fn new() -> Self {
        Self {
            search_queries: Vec::new(),
        }
    }
}

impl HtmlRetailerSuper for FrontierFirearms {}

#[async_trait]
impl Retailer for FrontierFirearms {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::FrontierFirearms
    }

    async fn init(&mut self) -> Result<(), RetailerError> {
        let queries = get_search_queries(SITE_MAP, PRODUCT_BASE_URL, |link| {
            if link.starts_with("optics/")
                || link.starts_with("shooting-accessories/")
                || link.contains("clothing/")
            {
                return Some(HtmlSearchQuery {
                    term: link,
                    category: Category::Other,
                });
            } else if (link.contains("ammunition.html") && !link.contains("rimfire"))
                || link.contains("shells.html")
            {
                // they don't really do ammo, but lets at least try
                return Some(HtmlSearchQuery {
                    term: link,
                    category: Category::Ammunition,
                });
            } else if link.starts_with("firearms.html") {
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
impl HtmlRetailer for FrontierFirearms {
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
        let bigcommerce_helper = BigCommerce::new();
        let mut results: Vec<CrawlResult> = Vec::new();

        let fragment = Html::parse_document(response);

        let product_selector = Selector::parse("ul.productGrid > li.product").unwrap();

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
        self.search_queries.clone()
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        BigCommerce::parse_max_pages(response)
    }
}
