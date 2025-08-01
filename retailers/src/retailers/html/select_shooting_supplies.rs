use std::pin::Pin;

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
        ecommerce::{bigcommerce::BigCommerce, bigcommerce_nested::BigCommerceNested},
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

const API_URL: &str =
    "https://selectshootingsupplies.com/remote/v1/product-attributes/{product_id}";
const CART_URL: &str = "https://selectshootingsupplies.com/cart.php";
const URL: &str = "https://selectshootingsupplies.com/{category}/?in_stock=1&page={page}";

pub struct SelectShootingSupplies;

impl SelectShootingSupplies {
    pub fn new() -> Self {
        Self {}
    }
}

impl HtmlRetailerSuper for SelectShootingSupplies {}

impl Retailer for SelectShootingSupplies {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::SelectShootingSupplies
    }
}

#[async_trait]
impl HtmlRetailer for SelectShootingSupplies {
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

        // commit another Rust sin, and clone the entire HTML
        // as a string since scraper::ElementRef is not thread safe
        // we'll recreate the Node later
        let products = {
            let html = Html::parse_document(response);
            let product_selector = Selector::parse("ul.productGrid > li.product").unwrap();
            html.select(&product_selector)
                .map(|element| element.html().clone())
                .collect::<Vec<_>>()
        };

        let mut nested_handlers: Vec<
            Pin<Box<dyn Future<Output = Result<Vec<CrawlResult>, RetailerError>> + Send>>,
        > = Vec::new();

        for html_doc in products {
            let product_inner = Html::parse_document(&html_doc);
            let product = product_inner.root_element();

            let price_element = extract_element_from_element(
                product,
                "div.price-section > span.price.price--withoutTax",
            )?;

            if element_to_text(price_element).contains("-") {
                let title_element = extract_element_from_element(product, "h4.card-title > a")?;
                let url = element_extract_attr(title_element, "href")?;

                // this is a nested firearm, there are models inside
                // the URL that have different prices
                nested_handlers.push(Box::pin(
                    BigCommerceNested::parse_nested(
                        API_URL,
                        url,
                        CART_URL,
                        self.get_retailer_name(),
                        search_term.category,
                        BigCommerce::get_item_name(product)?,
                        BigCommerce::get_image_url(product)?,
                    )
                    .into_future(),
                ));

                continue;
            }

            let result = BigCommerce::parse_product(
                product,
                self.get_retailer_name(),
                search_term.category,
            )?;

            results.push(result);
        }

        for handler in nested_handlers {
            results.append(&mut handler.await?);
        }

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        let mut terms = Vec::from_iter([HtmlSearchQuery {
            term: "firearms".into(),
            category: Category::Firearm,
        }]);

        let other_terms = [
            "firearm-parts-and-upgrades",
            "flashlights-and-laser-combos",
            "holsters-mag-pouches-and-speed-belts",
            "optics-sights-and-mounts",
            "range-gear",
            "reloading-1",
            "safety-personal-protection",
            "tools",
            "targets",
            "training-systems",
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
        BigCommerce::parse_max_pages(response)
    }
}
