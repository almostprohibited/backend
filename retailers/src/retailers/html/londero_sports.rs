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
    utils::{ecommerce::Magento, html::extract_element_from_element},
};

const URL: &str = "https://www.londerosports.com/en/{category}?p={page}&product_list_limit=80&product_list_order=quantity_and_stock_status";

pub struct LonderoSports {}

impl Default for LonderoSports {
    fn default() -> Self {
        Self::new()
    }
}

impl LonderoSports {
    pub fn new() -> Self {
        Self {}
    }
}

impl HtmlRetailerSuper for LonderoSports {}

#[async_trait]
impl Retailer for LonderoSports {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::LonderoSports
    }
}

#[async_trait]
impl HtmlRetailer for LonderoSports {
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
            if extract_element_from_element(element, "div.stock.unavailable").is_ok() {
                debug!("Out of stock item found, skipping");
                break;
            }

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

    // they have a sitemap, but I don't feel like parsing it
    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        let mut terms: Vec<HtmlSearchQuery> = vec![HtmlSearchQuery {
            term: "firearms/ammunitions".to_string(),
            category: Category::Ammunition,
        }];

        for firearm in [
            "firearms/muzzleloading/muzzleloaders",
            "firearms/rifles",
            "firearms/shotguns",
            "firearms/used-firearms",
        ] {
            terms.push(HtmlSearchQuery {
                term: firearm.to_string(),
                category: Category::Firearm,
            });
        }

        for other in [
            "firearms/gun-cases-storage",
            "firearms/magazines-accesories",
            "firearms/cleaning-care",
            "firearms/firearms-accessories",
            "firearms/shooting",
            "firearms/muzzleloading/powders",
            "firearms/muzzleloading/bullets-sabots-jackets",
            "firearms/muzzleloading/primers-caps",
            "firearms/muzzleloading/loading-accesories",
            "firearms/muzzleloading/tools",
            "firearms/muzzleloading/rods-ramrods",
            "firearms/muzzleloading/breech-plugs",
            "firearms/muzzleloading/cleaning-care",
            "reloading",
            "optics",
        ] {
            terms.push(HtmlSearchQuery {
                term: other.to_string(),
                category: Category::Other,
            });
        }

        terms
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        Ok(0)
        // let html = Html::parse_document(response);

        // if extract_element_from_element(
        //     html.root_element(),
        //     "ol.products > li.product div.stock.unavailable",
        // )
        // .is_ok()
        // {
        //     debug!("Out of stock item found, stopping category");
        //     return Ok(0);
        // }

        // Magento::get_pages(response)
    }
}
