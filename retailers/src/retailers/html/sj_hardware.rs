use async_trait::async_trait;
use common::result::{
    base::CrawlResult,
    enums::{Category, RetailerName},
};
use crawler::request::{Request, RequestBuilder};
use scraper::{Html, Selector};

use crate::{
    errors::RetailerError,
    structures::{HtmlRetailer, HtmlRetailerSuper, HtmlSearchQuery, Retailer},
    utils::{
        ecommerce::{BigCommerce, BigCommerceNested},
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

const SITE_URL: &str = "https://sjhardware.com/";
const URL: &str = "https://sjhardware.com/product-category/{category}/?page={page}&in_stock=1";

pub struct SJHardware;

impl Default for SJHardware {
    fn default() -> Self {
        Self::new()
    }
}

impl SJHardware {
    pub fn new() -> Self {
        Self {}
    }
}

impl HtmlRetailerSuper for SJHardware {}

impl Retailer for SJHardware {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::SJHardware
    }
}

#[async_trait]
impl HtmlRetailer for SJHardware {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &HtmlSearchQuery,
    ) -> Result<Request, RetailerError> {
        let url = URL
            .replace("{category}", &search_term.term)
            .replace("{page}", &(page_num + 1).to_string());

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

        let products = {
            let html = Html::parse_document(response);
            let product_selector = Selector::parse("ul.productGrid > li.product").unwrap();
            html.select(&product_selector)
                .map(|element| element.html().clone())
                .collect::<Vec<_>>()
        };

        for html_doc in products {
            let product_inner = Html::parse_document(&html_doc);
            let product = product_inner.root_element();

            let cart_button =
                extract_element_from_element(product, "div.card-text.add-to-cart-button")?;
            let button_text = element_to_text(cart_button).to_lowercase();

            let price_element = extract_element_from_element(
                product,
                "div.price-section > span.price.price--withoutTax",
            )?;
            let price_text = element_to_text(price_element);

            if button_text.contains("choose options") || price_text.contains("-") {
                let title_element = extract_element_from_element(product, "h4.card-title > a")?;
                let url = element_extract_attr(title_element, "href")?;

                // TODO: fix this, sj hardware has a product that is in stock, but
                // does not actually go anywhere when visited (it 404s)
                if !url.contains("https://sjhardware.com/6-israeli-bandages") {
                    let _ = bigcommerce_helper
                        .enqueue_nested_product_element(product, search_term.category);
                }
            } else if button_text.contains("add to cart") {
                let result = bigcommerce_helper.parse_product(
                    product,
                    self.get_retailer_name(),
                    search_term.category,
                )?;

                results.push(result);
            }
        }

        results.extend(
            bigcommerce_helper
                .parse_nested_products(SITE_URL, self.get_retailer_name())
                .await?,
        );

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        let mut terms: Vec<HtmlSearchQuery> = vec![];

        ["firearms"].iter().for_each(|search| {
            terms.push(HtmlSearchQuery {
                term: search.to_string(),
                category: Category::Firearm,
            });
        });

        // https://sjhardware.com/product-category/ammo/
        // their ammo page says they don't ship ammo
        // oh well

        [
            "gun-cleaning",
            "tactical-gear",
            "medical-and-survival",
            "milsurp",
            "optics",
            "accessories",
            "precision-rifle-components",
            "reloading-components",
        ]
        .iter()
        .for_each(|search| {
            terms.push(HtmlSearchQuery {
                term: search.to_string(),
                category: Category::Other,
            });
        });

        terms
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        BigCommerce::parse_max_pages(response)
    }
}
