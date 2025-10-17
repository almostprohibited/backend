use std::{collections::HashMap, time::Duration};

use common::{
    constants::CRAWL_COOLDOWN_SECS,
    result::{
        base::{CrawlResult, Price},
        enums::{Category, RetailerName},
    },
};
use crawler::{request::RequestBuilder, unprotected::UnprotectedCrawler};
use scraper::{Html, Selector};
use serde::Deserialize;
use tokio::time::sleep;

use crate::{
    errors::RetailerError,
    utils::{
        conversions::price_to_cents,
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

struct NestedProduct {
    url: String,
    category: Category,
}

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

pub(crate) struct WooCommerceNested {
    crawler: UnprotectedCrawler,
    url_queue: Vec<NestedProduct>,
    retailer_name: RetailerName,
}

impl WooCommerceNested {
    pub(crate) fn new(retailer: RetailerName) -> Self {
        Self {
            crawler: UnprotectedCrawler::new(),
            url_queue: Vec::new(),
            retailer_name: retailer,
        }
    }

    pub(crate) fn enqueue_product(&mut self, url: String, category: Category) {
        self.url_queue.push(NestedProduct { url, category });
    }

    fn get_product_variations(
        result: &String,
        product_url: &String,
    ) -> Result<Vec<ProductVariation>, RetailerError> {
        let html = Html::parse_document(result);

        let form_element = extract_element_from_element(
            html.root_element(),
            format!("form[action='{product_url}']"),
        )?;
        let form_attribute = element_extract_attr(form_element, "data-product_variations")?;

        Ok(serde_json::from_str::<Vec<ProductVariation>>(
            &form_attribute,
        )?)
    }

    fn get_nested_product_title(result: &String) -> Result<String, RetailerError> {
        let html = Html::parse_document(result);
        let title = extract_element_from_element(html.root_element(), "h1.product_title")?;

        Ok(element_to_text(title))
    }

    fn get_product_attribute_name_mapping(
        result: &String,
        variations: &Vec<ProductVariation>,
    ) -> Result<HashMap<String, HashMap<String, String>>, RetailerError> {
        let html = Html::parse_document(result);

        let mut results: HashMap<String, HashMap<String, String>> = HashMap::new();

        for variation in variations {
            for attribute in variation.attributes.keys() {
                if results.contains_key(attribute) {
                    continue;
                }

                let mut mapping: HashMap<String, String> = HashMap::new();

                let selector =
                    Selector::parse(&format!("li[data-attribute_name='{attribute}'")).unwrap();

                for attribute in html.select(&selector) {
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

            // special handling for Rangeview Sports
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

    pub(crate) async fn parse_nested(&self) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut results: Vec<CrawlResult> = Vec::new();

        for nested_product in &self.url_queue {
            let request = RequestBuilder::new().set_url(&nested_product.url).build();
            let result = self.crawler.make_web_request(request).await?;

            let product_title = Self::get_nested_product_title(&result.body)?;

            let product_variations =
                Self::get_product_variations(&result.body, &nested_product.url)?;

            let attribute_mapping =
                Self::get_product_attribute_name_mapping(&result.body, &product_variations)?;

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

                let new_result = CrawlResult::new(
                    name,
                    nested_product.url.clone(),
                    price,
                    self.retailer_name,
                    nested_product.category,
                )
                .with_image_url(variation.image.url);

                results.push(new_result);
            }

            sleep(Duration::from_secs(CRAWL_COOLDOWN_SECS)).await;
        }

        Ok(results)
    }
}
