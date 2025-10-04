use async_trait::async_trait;
use common::result::{
    base::CrawlResult,
    enums::{Category, RetailerName},
};
use crawler::request::{Request, RequestBuilder};
use scraper::{ElementRef, Html, Selector};
use tracing::debug;

use crate::{
    errors::RetailerError,
    structures::{HtmlRetailer, HtmlRetailerSuper, HtmlSearchQuery, Retailer},
    utils::{
        ecommerce::{
            woocommerce::{WooCommerce, WooCommerceBuilder},
            woocommerce_nested::WooCommerceNested,
        },
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

const MAX_PER_PAGE: &str = "20";
const URL: &str = "https://www.rangeviewsports.ca/product-category/{category}/page/{page}/?per_page={max_per_page}";

const BLOCKED_TITLE_TERMS: [&str; 2] = ["special order*", "*in store only*"];

pub struct RangeviewSports {}

impl Default for RangeviewSports {
    fn default() -> Self {
        Self::new()
    }
}

impl RangeviewSports {
    pub fn new() -> Self {
        Self {}
    }

    fn is_out_of_stock(element: ElementRef) -> bool {
        extract_element_from_element(element, "span.out-of-stock.product-label").is_ok()
    }
}

impl HtmlRetailerSuper for RangeviewSports {}

impl Retailer for RangeviewSports {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::RangeviewSports
    }
}

#[async_trait]
impl HtmlRetailer for RangeviewSports {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &HtmlSearchQuery,
    ) -> Result<Request, RetailerError> {
        let url = URL
            .replace("{category}", &search_term.term)
            .replace("{page}", &(page_num + 1).to_string())
            .replace("{max_per_page}", MAX_PER_PAGE);

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

        let woocommerce_helper = WooCommerceBuilder::default().build();
        let mut woocommerce_nested = WooCommerceNested::new(self.get_retailer_name());

        let products = {
            let html = Html::parse_document(response);
            let product_selector =
                Selector::parse("div.products > div.product > div.product-wrapper").unwrap();
            html.select(&product_selector)
                .map(|element| element.html().clone())
                .collect::<Vec<_>>()
        };

        for doc in products {
            let product_inner = Html::parse_fragment(&doc);
            let product = product_inner.root_element();

            if Self::is_out_of_stock(product) {
                break;
            }

            // leave this outside and not inside if-let-Ok statement to fail on purpose
            let price_element = extract_element_from_element(product, "span.price")?;
            let link_element = extract_element_from_element(product, "h3.wd-entities-title > a")?;

            let name = element_to_text(link_element).to_lowercase();

            if BLOCKED_TITLE_TERMS
                .iter()
                .any(|term| name.contains(&term.to_lowercase()))
            {
                continue;
            };

            // rangeview does something dumb and uses a unicode dash
            // to show case price range, instead of regular ascii
            //
            // so what I have in the contains below IS A UNICODE DASH
            if element_to_text(price_element).contains("–") {
                let link = element_extract_attr(link_element, "href")?;

                woocommerce_nested.enqueue_product(link, search_term.category.clone());

                continue;
            };

            let result = woocommerce_helper.parse_product(
                product,
                self.get_retailer_name(),
                search_term.category,
            )?;

            results.push(result);
        }

        results.extend(woocommerce_nested.parse_nested().await?);

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        let mut terms = Vec::from_iter([
            HtmlSearchQuery {
                term: "firearms".into(),
                category: Category::Firearm,
            },
            HtmlSearchQuery {
                term: "preowned".into(),
                category: Category::Firearm,
            },
        ]);

        let ammo_terms = [
            "ammo/rimfire-ammo",
            "ammo/rifle-ammo",
            "ammo/handgun-ammo",
            "ammo/shotgun-ammo",
            "ammo/bulk-ammo",
        ];

        for ammo in ammo_terms {
            terms.push(HtmlSearchQuery {
                term: ammo.into(),
                category: Category::Ammunition,
            });
        }

        let other_terms = [
            "reloading",
            "optics",
            "firearm-accessories",
            "shooting-range-accessories",
        ];

        for other in other_terms {
            terms.push(HtmlSearchQuery {
                term: other.into(),
                category: Category::Other,
            });
        }

        terms
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        let html = Html::parse_document(response);

        if Self::is_out_of_stock(html.root_element()) {
            return Ok(0);
        }

        WooCommerce::parse_max_pages(response)
    }
}
