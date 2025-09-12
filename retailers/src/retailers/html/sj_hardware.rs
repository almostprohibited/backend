use async_trait::async_trait;
use common::result::{
    base::CrawlResult,
    enums::{Category, RetailerName},
};
use crawler::request::{Request, RequestBuilder};
use scraper::{ElementRef, Html, Selector};

use crate::{
    errors::RetailerError,
    structures::{HtmlRetailer, HtmlRetailerSuper, HtmlSearchQuery, Retailer},
    utils::{
        ecommerce::{
            bigcommerce::BigCommerce,
            bigcommerce_nested::{BigCommerceNested, NestedProduct},
        },
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

const API_URL: &str = "https://sjhardware.com/remote/v1/product-attributes/{product_id}";
const CART_URL: &str = "https://sjhardware.com/cart.php";
const URL: &str = "https://sjhardware.com/product-category/{category}/?page={page}&in_stock=1";

pub struct SJHardware;

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
        let mut nested_handler =
            BigCommerceNested::new(API_URL, CART_URL, self.get_retailer_name());

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

            let cart_button_text =
                extract_element_from_element(product, "div.card-text > a.button")?;

            if element_to_text(cart_button_text)
                .to_lowercase()
                .contains("choose options")
            {
                let title_element = extract_element_from_element(product, "h4.card-title > a")?;
                let url = element_extract_attr(title_element, "href")?;

                nested_handler.enqueue_product(NestedProduct {
                    name: BigCommerce::get_item_name(product)?,
                    fallback_image_url: BigCommerce::get_image_url(product)?,
                    category: search_term.category,
                    product_url: url,
                });
            } else {
                let result = BigCommerce::parse_product(
                    product,
                    self.get_retailer_name(),
                    search_term.category,
                )?;

                results.push(result);
            }
        }

        results.extend(nested_handler.parse_nested().await?);

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
