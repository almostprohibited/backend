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
    traits::{Retailer, SearchTerm},
    utils::ecommerce::bigcommerce::BigCommerce,
};

const URL: &str = "https://selectshootingsupplies.com/{category}/?in_stock=1&page={page}";

pub struct SelectShootingSupplies {
    retailer: RetailerName,
}

impl SelectShootingSupplies {
    pub fn new() -> Self {
        Self {
            retailer: RetailerName::SelectShootingSupplies,
        }
    }
}

#[async_trait]
impl Retailer for SelectShootingSupplies {
    fn get_retailer_name(&self) -> RetailerName {
        self.retailer
    }

    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &SearchTerm,
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
        search_term: &SearchTerm,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut results: Vec<CrawlResult> = Vec::new();

        let html = Html::parse_document(&response);

        let product_selector = Selector::parse("ul.productGrid > li.product").unwrap();

        for product in html.select(&product_selector) {
            let result = BigCommerce::parse_product(
                product,
                self.get_retailer_name(),
                search_term.category,
            )?;

            results.push(result);
        }

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<SearchTerm> {
        Vec::from_iter([
            SearchTerm {
                term: "firearms".into(),
                category: Category::Firearm,
            },
            SearchTerm {
                term: "firearm-parts-and-upgrades".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "flashlights-and-laser-combos".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "holsters-mag-pouches-and-speed-belts".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "optics-sights-and-mounts".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "range-gear".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "reloading-1".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "safety-personal-protection".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "tools".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "targets".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "training-systems".into(),
                category: Category::Other,
            },
        ])
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        BigCommerce::parse_max_pages(response)
    }
}
