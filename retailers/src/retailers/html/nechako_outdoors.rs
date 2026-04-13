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
        ecommerce::{BigCommerce, BigCommerceNested},
        generic_sitemap::get_search_queries,
        html::{element_to_text, extract_element_from_element},
    },
};

const SITE_URL: &str = "https://nechakooutdoors.ca/";
const SITE_MAP: &str = "https://nechakooutdoors.ca/xmlsitemap.php?type=categories&page=1";
const URL: &str = "https://nechakooutdoors.ca/{category}/?in_stock=1&page={page}";

pub struct NechakoOutdoors {
    search_queries: Vec<HtmlSearchQuery>,
}

impl Default for NechakoOutdoors {
    fn default() -> Self {
        Self::new()
    }
}

impl NechakoOutdoors {
    pub fn new() -> Self {
        Self {
            search_queries: Vec::new(),
        }
    }
}

impl HtmlRetailerSuper for NechakoOutdoors {}

#[async_trait]
impl Retailer for NechakoOutdoors {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::NechakoOutdoors
    }

    async fn init(&mut self) -> Result<(), RetailerError> {
        let queries = get_search_queries(SITE_MAP, SITE_URL, |link| {
            if link == "firearms" {
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

            if ["optics", "reloading"].contains(&link.as_str()) {
                return Some(HtmlSearchQuery {
                    term: link,
                    category: Category::Other,
                });
            }

            if link.starts_with("shooting/") && !link.contains("air-rifles") {
                return Some(HtmlSearchQuery {
                    term: link,
                    category: Category::Other,
                });
            }

            None
        })
        .await?;

        self.search_queries.extend(queries);

        Ok(())
    }
}

#[async_trait]
impl HtmlRetailer for NechakoOutdoors {
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
        let mut bigcommerce_helper = BigCommerce::new();
        let mut results: Vec<CrawlResult> = Vec::new();

        let fragment = Html::parse_document(response);

        let product_selector = Selector::parse("ul.productGrid > li.product").unwrap();

        for element in fragment.select(&product_selector) {
            // they have products listed as "in stock" while being out of stock
            let Ok(cart_button) = extract_element_from_element(
                element,
                "div.card-figcaption--action-buttons > a[data-event-type='product-click']",
            ) else {
                debug!("Item listed as in stock that is out of stock");

                continue;
            };

            let cart_text = element_to_text(cart_button).to_lowercase();

            let price_element = extract_element_from_element(
                element,
                "div.price-section > span.price.price--withoutTax",
            )?;

            let price_text = element_to_text(price_element);

            if cart_text == "view options" && price_text.contains("-") {
                let _ = bigcommerce_helper
                    .enqueue_nested_product_element(element, search_term.category);

                continue;
            }

            let result = bigcommerce_helper.parse_product(
                element,
                self.get_retailer_name(),
                search_term.category,
            )?;

            results.push(result);
        }

        results.extend(
            bigcommerce_helper
                .parse_nested_products(SITE_URL, self.get_retailer_name())
                .await?,
        );

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        self.search_queries.clone()
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        BigCommerce::parse_max_pages(response)
    }
}
