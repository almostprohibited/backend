use async_trait::async_trait;
use common::result::{
    base::CrawlResult,
    enums::{Category, RetailerName},
};
use crawler::request::{Request, RequestBuilder};
use scraper::Html;
use tracing::debug;

use crate::{
    errors::RetailerError,
    structures::{HtmlRetailer, HtmlRetailerSuper, HtmlSearchQuery, Retailer},
    utils::{
        ecommerce::Odoo, generic_sitemap::get_search_queries, html::extract_element_from_element,
    },
};

const BASE_URL: &str = "https://outfitters.goldnloan.com/";
const PRODUCT_BASE_URL: &str = "https://outfitters.goldnloan.com/shop/";
const SITE_MAP: &str = "https://outfitters.goldnloan.com/sitemap.xml";
const URL: &str = "https://outfitters.goldnloan.com/shop/{category}/page/{page}?order=&view_mode=grid&attrib=&attrib=&attrib=&hide_out_of_stock=1";
const QUICK_VIEW_URL: &str = "https://outfitters.goldnloan.com/theme_prime/get_quick_view_html";
const VARIANT_URL: &str = "https://outfitters.goldnloan.com/website_sale/get_combination_info";

pub struct GoldNLoan {
    search_queries: Vec<HtmlSearchQuery>,
}

impl Default for GoldNLoan {
    fn default() -> Self {
        Self::new()
    }
}

impl GoldNLoan {
    pub fn new() -> Self {
        Self {
            search_queries: Vec::new(),
        }
    }
}

impl HtmlRetailerSuper for GoldNLoan {}

#[async_trait]
impl Retailer for GoldNLoan {
    async fn init(&mut self) -> Result<(), RetailerError> {
        let queries = get_search_queries(SITE_MAP, PRODUCT_BASE_URL, |link| {
            if !link.starts_with("category/") {
                return None;
            }

            if link.contains("/ammunition-") {
                return Some(HtmlSearchQuery {
                    term: link,
                    category: Category::Ammunition,
                });
            } else if link.contains("/firearms-")
                && link.chars().filter(|char| *char == '-').count() == 1
            {
                return Some(HtmlSearchQuery {
                    term: link,
                    category: Category::Firearm,
                });
            };

            Some(HtmlSearchQuery {
                term: link,
                category: Category::Other,
            })
        })
        .await?;

        self.search_queries.extend(queries);

        Ok(())
    }

    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::GoldNLoan
    }
}

#[async_trait]
impl HtmlRetailer for GoldNLoan {
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
        let odoo = Odoo::new(
            BASE_URL,
            QUICK_VIEW_URL,
            VARIANT_URL,
            self.get_retailer_name(),
        );
        Ok(odoo.parse_page(response, search_term).await?)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        self.search_queries.clone()
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        let html = Html::parse_document(response);

        match extract_element_from_element(html.root_element(), "a.tp-load-more-btn") {
            Ok(_) => Ok(u64::MAX),
            Err(_) => Ok(0),
        }
    }
}
