use std::{collections::HashMap, pin::Pin, time::Duration};

use async_trait::async_trait;
use common::{
    result::{
        base::{CrawlResult, Price},
        enums::{Category, RetailerName},
    },
    utils::CRAWL_COOLDOWN_SECS,
};
use crawler::{
    request::{Request, RequestBuilder},
    unprotected::UnprotectedCrawler,
};
use scraper::{ElementRef, Html, Selector};
use serde::Deserialize;
use tokio::time::sleep;
use tracing::debug;

use crate::{
    errors::RetailerError,
    structures::{HtmlRetailer, HtmlRetailerSuper, HtmlSearchQuery, Retailer},
    utils::{
        conversions::price_to_cents,
        ecommerce::woocommerce::{WooCommerce, WooCommerceBuilder},
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

const MAX_PER_PAGE: &str = "20";
const URL: &str = "https://www.rangeviewsports.ca/product-category/{category}/page/{page}/?per_page={max_per_page}";

#[derive(Deserialize, Debug)]
struct ProductImage {
    url: String,
}

#[derive(Deserialize, Debug)]
struct ProductVariation {
    attributes: HashMap<String, String>,
    image: ProductImage,
    is_in_stock: bool,
    display_price: f32,
    display_regular_price: f32,
}

pub struct RangeviewSports {
    crawler: UnprotectedCrawler,
}

impl Default for RangeviewSports {
    fn default() -> Self {
        Self::new()
    }
}

impl RangeviewSports {
    pub fn new() -> Self {
        Self {
            crawler: UnprotectedCrawler::new(),
        }
    }

    fn is_out_of_stock(element: ElementRef) -> bool {
        extract_element_from_element(element, "span.out-of-stock.product-label").is_ok()
    }

    fn get_product_variations(
        element: ElementRef,
        product_url: &String,
    ) -> Result<Vec<ProductVariation>, RetailerError> {
        let form_element =
            extract_element_from_element(element, format!("form[action='{product_url}']"))?;
        let form_attribute = element_extract_attr(form_element, "data-product_variations")?;

        Ok(serde_json::from_str::<Vec<ProductVariation>>(
            &form_attribute,
        )?)
    }

    fn get_nested_product_title(element: ElementRef) -> Result<String, RetailerError> {
        let title = extract_element_from_element(element, "h1.product_title")?;

        Ok(element_to_text(title))
    }

    fn get_product_attribute_name_mapping(
        element: ElementRef,
        variations: &Vec<ProductVariation>,
    ) -> Result<HashMap<String, HashMap<String, String>>, RetailerError> {
        let mut results: HashMap<String, HashMap<String, String>> = HashMap::new();

        for variation in variations {
            for attribute in variation.attributes.keys() {
                if results.contains_key(attribute) {
                    continue;
                }

                let mut mapping: HashMap<String, String> = HashMap::new();

                let selector =
                    Selector::parse(&format!("li[data-attribute_name='{attribute}'")).unwrap();

                for attribute in element.select(&selector) {
                    let attr_key = element_extract_attr(attribute, "data-value")?;
                    let attr_name = element_extract_attr(attribute, "title")?;

                    mapping.insert(attr_key, attr_name);
                }

                results.insert(attribute.to_string(), mapping);
            }
        }

        Ok(results)
    }

    // I don't like how this returns a Result<Option<String>>
    // this is "temporary" to fix extra product issue
    fn format_nested_name(
        product_title: &String,
        variation: &ProductVariation,
        attribute_mapping: &HashMap<String, HashMap<String, String>>,
    ) -> Result<Option<String>, RetailerError> {
        let mut attribute_names: Vec<String> = Vec::new();

        for (variation_attr_key, variation_attr_value) in &variation.attributes {
            if variation_attr_key.is_empty() || variation_attr_value.is_empty() {
                continue;
            }

            let Some(mapping) = attribute_mapping.get(variation_attr_key) else {
                return Err(RetailerError::HtmlMissingElement(format!(
                    "'attribute {variation_attr_key} is missing'"
                )));
            };

            let Some(attr_name) = mapping.get(variation_attr_value) else {
                // return Err(RetailerError::HtmlMissingElement(format!(
                //     "'attribute {variation_attr_key} is missing value {variation_attr_value}'"
                // )));

                // oddly enough, Rangeview Sports will include items
                // in their API response that are "in stock", but
                // don't show up on the website
                return Ok(None);
            };

            attribute_names.push(attr_name.clone());
        }

        let flat_attr_names = attribute_names.join(" - ");

        Ok(Some(format!("{product_title} - {flat_attr_names}")))
    }

    async fn parse_nested(
        &self,
        url: String,
        category: Category,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut results: Vec<CrawlResult> = Vec::new();

        let request = RequestBuilder::new().set_url(&url).build();
        let result = self.crawler.make_web_request(request).await?;

        let html = Html::parse_document(&result.body);
        let root_element = html.root_element();

        let product_title = Self::get_nested_product_title(root_element)?;

        let product_variations = Self::get_product_variations(root_element, &url)?;

        let attribute_mapping =
            Self::get_product_attribute_name_mapping(root_element, &product_variations)?;

        for variation in product_variations {
            if !variation.is_in_stock {
                continue;
            }

            let regular_price = price_to_cents(variation.display_regular_price.to_string())?;
            let sale_price = price_to_cents(variation.display_price.to_string())?;

            let price = Price {
                regular_price,
                sale_price: if regular_price == sale_price {
                    None
                } else {
                    Some(sale_price)
                },
            };

            let Some(name) =
                Self::format_nested_name(&product_title, &variation, &attribute_mapping)?
            else {
                // none indicating extra product that is not
                // shown to public
                continue;
            };

            let new_result =
                CrawlResult::new(name, url.clone(), price, self.get_retailer_name(), category)
                    .with_image_url(variation.image.url);

            results.push(new_result);
        }

        Ok(results)
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

        // commit another Rust sin, and clone the entire HTML
        // as a string since scraper::ElementRef is not thread safe
        // we'll recreate the Node later
        let products = {
            let html = Html::parse_document(response);
            let product_selector =
                Selector::parse("div.products > div.product > div.product-wrapper").unwrap();
            html.select(&product_selector)
                .map(|element| element.html().clone())
                .collect::<Vec<_>>()
        };

        let mut nested_handlers: Vec<
            Pin<Box<dyn Future<Output = Result<Vec<CrawlResult>, RetailerError>> + Send>>,
        > = Vec::new();

        for doc in products {
            let product_inner = Html::parse_fragment(&doc);
            let product = product_inner.root_element();

            if Self::is_out_of_stock(product) {
                break;
            }

            // leave this outside and not inside if-let-Ok statement to fail on purpose
            let price_element = extract_element_from_element(product, "span.price")?;
            let link_element = extract_element_from_element(product, "h3.wd-entities-title > a")?;

            if element_to_text(link_element)
                .to_lowercase()
                .starts_with("*in store only*")
            {
                continue;
            }

            // rangeview does something dumb and uses a unicode dash
            // to show case price range, instead of regular ascii
            //
            // so what I have in the contains below IS A UNICODE DASH
            if element_to_text(price_element).contains("–") {
                let link = element_extract_attr(link_element, "href")?;

                nested_handlers.push(Box::pin(
                    self.parse_nested(link, search_term.category).into_future(),
                ));

                continue;
            };

            let result = woocommerce_helper.parse_product(
                product,
                self.get_retailer_name(),
                search_term.category,
            )?;

            results.push(result);
        }

        for handler in nested_handlers {
            results.append(&mut handler.await?);

            sleep(Duration::from_secs(CRAWL_COOLDOWN_SECS)).await;
        }

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
