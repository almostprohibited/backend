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
    utils::{auctollo_sitemap::get_search_queries, ecommerce::woocommerce::WooCommerceBuilder},
};

const SITE_MAP_URL: &str = "https://marstar.ca/productcat-sitemap.xml";
const PRODUCT_BASE_URL: &str = "https://marstar.ca/product-category/";
const URL: &str = "https://marstar.ca/product-category/{category}/?in_stock=1";

pub struct Marstar {
    search_terms: Vec<HtmlSearchQuery>,
}

impl Marstar {
    pub fn new() -> Self {
        Self {
            search_terms: Vec::new(),
        }
    }

    async fn get_search_queries() -> Result<Vec<HtmlSearchQuery>, RetailerError> {
        get_search_queries(SITE_MAP_URL, PRODUCT_BASE_URL, |link| {
            if link.starts_with("accessories/")
                || link.starts_with("reloading/")
                || link.starts_with("optic/")
                || link.starts_with("firearm-parts-and-accessories/")
                || link.starts_with("ammunition/reloading-supplies/")
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

impl HtmlRetailerSuper for Marstar {}

#[async_trait]
impl Retailer for Marstar {
    async fn init(&mut self) -> Result<(), RetailerError> {
        self.search_terms.extend(Self::get_search_queries().await?);
        Ok(())
    }

    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::Marstar
    }
}

#[async_trait]
impl HtmlRetailer for Marstar {
    async fn build_page_request(
        &self,
        _page_num: u64,
        search_term: &HtmlSearchQuery,
    ) -> Result<Request, RetailerError> {
        let url = URL.replace("{category}", &search_term.term);

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

        let html = Html::parse_document(&response);
        let product_selector =
            Selector::parse("ul.products > li.product.product-type-simple.instock.purchasable")
                .unwrap();

        let woocommerce_helper = WooCommerceBuilder::default()
            .with_product_url_selector("div.woocommerce-loop-product__title > a")
            .with_product_name_selector("div.woocommerce-loop-product__title > a")
            .with_image_url_selector(
                "div.woocommerce-image__wrapper > a.woocommerce-LoopProduct-link img",
            )
            .build();

        for product in html.select(&product_selector) {
            results.push(woocommerce_helper.parse_product(
                product,
                self.get_retailer_name(),
                search_term.category,
            )?);
        }

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        self.search_terms.clone()
    }

    fn get_num_pages(&self, _response: &String) -> Result<u64, RetailerError> {
        Ok(0)
    }
}
