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
    utils::ecommerce::{WooCommerce, WooCommerceBuilder},
};

const URL: &str = "https://pdent.ca/category/product-category/{category}/page/{page}/?filter_stock_status=instock";

pub struct PDEnterprises {}

impl Default for PDEnterprises {
    fn default() -> Self {
        Self::new()
    }
}

impl PDEnterprises {
    pub fn new() -> Self {
        Self {}
    }
}

impl HtmlRetailerSuper for PDEnterprises {}

impl Retailer for PDEnterprises {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::PDEnterprises
    }
}

#[async_trait]
impl HtmlRetailer for PDEnterprises {
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

        let product_selector = Selector::parse("li.product").unwrap();

        let woocommerce_helper = WooCommerceBuilder::default()
            .with_product_url_selector("a.wc-block-grid__product-link")
            .with_product_name_selector(
                "a.wc-block-grid__product-link > h3.wc-block-grid__product-title",
            )
            .with_image_url_selector("a.wc-block-grid__product-link img")
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

    // they don't have a usable sitemap with category information
    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        let mut terms: Vec<HtmlSearchQuery> = vec![HtmlSearchQuery {
            term: "ammunition".to_string(),
            category: Category::Ammunition,
        }];

        for firearm_category in ["firearms-used", "firearms-new"] {
            terms.push(HtmlSearchQuery {
                term: firearm_category.to_string(),
                category: Category::Firearm,
            });
        }

        for other in [
            "optics",
            "components",
            "reloading",
            "cleaning",
            "gun-parts",
            "knives",
            "lights",
            "media",
            "miscellaneous",
            "shooting-accessories",
            "storage",
            "targets",
            "tools",
        ] {
            terms.push(HtmlSearchQuery {
                term: other.to_string(),
                category: Category::Other,
            });
        }

        terms
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        WooCommerce::parse_max_pages(response)
    }
}
