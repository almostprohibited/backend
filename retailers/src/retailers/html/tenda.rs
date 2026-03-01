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
        securi_cookie::get_securi_cookie,
    },
};

const SITE_MAP: &str = "https://www.gotenda.com/product_cat-sitemap.xml";
const PRODUCT_BASE_URL: &str = "https://www.gotenda.com/product-category/";
const BASE_URL: &str = "https://www.gotenda.com/";
const URL: &str = "https://www.gotenda.com/product-category/{category}/page/{page}/?stock=instock";

pub struct Tenda {
    securi_cookie: String,
    search_terms: Vec<HtmlSearchQuery>,
}

impl Default for Tenda {
    fn default() -> Self {
        Self::new()
    }
}

impl Tenda {
    pub fn new() -> Self {
        Self {
            securi_cookie: String::new(),
            search_terms: Vec::new(),
        }
    }

    async fn get_search_queries() -> Result<Vec<HtmlSearchQuery>, RetailerError> {
        get_search_queries(SITE_MAP, PRODUCT_BASE_URL, |link| {
            if link.contains("/watches") || link.contains("/casual") || link.contains("/hats") {
                return None;
            }

            if link.starts_with("accessories/")
                || link.starts_with("reloading/")
                || link.starts_with("optic/")
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
        .await
    }
}

impl HtmlRetailerSuper for Tenda {}

#[async_trait]
impl Retailer for Tenda {
    async fn init(&mut self) -> Result<(), RetailerError> {
        let cookie = get_securi_cookie(BASE_URL).await?;

        debug!("Using cookie: {cookie}");

        self.securi_cookie = cookie;
        self.search_terms.extend(Self::get_search_queries().await?);

        Ok(())
    }

    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::Tenda
    }
}

#[async_trait]
impl HtmlRetailer for Tenda {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &HtmlSearchQuery,
    ) -> Result<Request, RetailerError> {
        let url = URL
            .replace("{category}", &search_term.term)
            .replace("{page}", &(page_num + 1).to_string());

        debug!("Setting page to {}", url);

        let request = RequestBuilder::new()
            .set_url(url)
            .set_headers([("Cookie".into(), self.securi_cookie.clone())].as_ref())
            .build();

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

        let woocommerce_helper = WooCommerceBuilder::default()
            .with_product_url_selector("h3.products-title > a")
            .with_product_name_selector("h3.products-title > a")
            .with_image_url_selector("figure.products-img > a > img")
            .build();

        for element in fragment.select(&product_selector) {
            results.push(woocommerce_helper.parse_product(
                element,
                self.get_retailer_name(),
                search_term.category,
            )?);
        }

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        // sucks to clone this, but I don't remember if this is run in a loop
        self.search_terms.clone()
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        WooCommerce::parse_max_pages(response)
    }
}
