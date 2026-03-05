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
    "https://thegundealer.ca/product-category/{category}/page/{page}/?filter_stock_status=instock";

const PRODUCT_BASE_URL: &str = "https://thegundealer.ca/product-category/";

const SITEMAPS: [&str; 2] = [
    "https://thegundealer.ca/product_cat-sitemap1.xml",
    "https://thegundealer.ca/product_cat-sitemap2.xml",
];

pub struct TheGunDealer {
    search_queries: Vec<HtmlSearchQuery>,
}

impl Default for TheGunDealer {
    fn default() -> Self {
        Self::new()
    }
}

impl TheGunDealer {
    pub fn new() -> Self {
        Self {
            search_queries: Vec::new(),
        }
    }
}

impl HtmlRetailerSuper for TheGunDealer {}

#[async_trait]
impl Retailer for TheGunDealer {
    async fn init(&mut self) -> Result<(), RetailerError> {
        for sitemap_url in SITEMAPS {
            let queries = get_search_queries(sitemap_url, PRODUCT_BASE_URL, |link| {
                if [
                    "reloading-components",
                    "optics",
                    "accessories/range-accessories",
                    "accessories/firearm-care",
                    "accessories/firearms-accessories",
                    "parts",
                    "accessories/storage-solutions",
                    "used-items/used-optics",
                    "used-items/used-parts",
                ]
                .contains(&link.as_str())
                {
                    return Some(HtmlSearchQuery {
                        term: link,
                        category: Category::Other,
                    });
                } else if link == "ammunition" {
                    return Some(HtmlSearchQuery {
                        term: link,
                        category: Category::Ammunition,
                    });
                } else if ["fine-guns", "new-guns", "used-items/used-guns"].contains(&link.as_str())
                {
                    return Some(HtmlSearchQuery {
                        term: link,
                        category: Category::Firearm,
                    });
                };

                None
            })
            .await?;

            self.search_queries.extend(queries);
        }

        println!("{:?}", self.search_queries);

        Ok(())
    }

    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::TheGunDealer
    }
}

#[async_trait]
impl HtmlRetailer for TheGunDealer {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &HtmlSearchQuery,
    ) -> Result<Request, RetailerError> {
        let url = URL
            .replace("{category}", &search_term.term)
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
        let mut results: Vec<CrawlResult> = Vec::new();

        let html = Html::parse_document(response);

        let product_selector = Selector::parse("ul.products > li.product.instock").unwrap();

        let woocommerce_helper = WooCommerceBuilder::default()
            .with_image_url_selector("div.product-thumbnail img")
            .build();

        for product in html.select(&product_selector) {
            let new_product = woocommerce_helper.parse_product(
                product,
                self.get_retailer_name(),
                search_term.category,
            )?;

            results.push(new_product);
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
