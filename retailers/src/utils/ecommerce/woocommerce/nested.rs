use std::{collections::HashMap, time::Duration};

use async_trait::async_trait;
use common::{
    constants::CRAWL_COOLDOWN_SECS,
    result::{
        base::{CrawlResult, Price},
        enums::{Category, RetailerName},
    },
};
use crawler::{WebClient, request::RequestBuilder};
use scraper::{Html, Selector};
use tokio::time::sleep;
use tracing::{debug, warn};

use crate::{
    errors::RetailerError,
    utils::{
        conversions::price_to_cents,
        ecommerce::{WooCommerce, woocommerce::structs::ProductVariation},
        html::{
            element_extract_attr, element_to_text, extract_element_from_element,
            match_element_from_list,
        },
    },
};

const DEFAULT_TITLE_SELECTORS: [&str; 2] = ["h1.product_title", "h3.page-title"];

#[async_trait]
pub(crate) trait WooCommerceNested {
    async fn parse_nested_products(
        url: String,
        category: Category,
        retailer_name: RetailerName,
    ) -> Result<Vec<CrawlResult>, RetailerError>;
}

impl WooCommerce {
    fn get_nested_product_variations(
        result: &String,
        product_url: &String,
    ) -> Result<Vec<ProductVariation>, RetailerError> {
        let html = Html::parse_document(result);

        let form_element = extract_element_from_element(
            html.root_element(),
            format!("form[action='{product_url}']"),
        )?;
        let form_attribute = element_extract_attr(form_element, "data-product_variations")?;

        if form_attribute == "false" {
            // TODO: handle shooter choice/woocommerce? properly
            // they appear to have a "proper" endpoint for this
            //
            // https://shooterschoice.com/?wc-ajax=get_variation

            return Ok(vec![]);
        }

        Ok(serde_json::from_str::<Vec<ProductVariation>>(
            &form_attribute,
        )?)
    }

    fn get_nested_product_title(result: &String) -> Result<String, RetailerError> {
        let html = Html::parse_document(result);

        let title = match_element_from_list(
            html.root_element(),
            &DEFAULT_TITLE_SELECTORS
                .iter()
                .map(|selector| selector.to_string())
                .collect(),
            RetailerError::HtmlMissingElement("Missing nested title element".to_string()),
        )?;

        Ok(element_to_text(title))
    }

    fn get_nested_product_attribute_name_mapping(
        result: &String,
        variations: &Vec<ProductVariation>,
    ) -> Result<HashMap<String, HashMap<String, String>>, RetailerError> {
        let html = Html::parse_document(result);

        let mut results: HashMap<String, HashMap<String, String>> = HashMap::new();

        for variation in variations {
            for attribute in variation.attributes.keys() {
                if results.contains_key(attribute) {
                    debug!("Results already contain {attribute}, skipping");
                    continue;
                }

                let mut mapping: HashMap<String, String> = HashMap::new();

                debug!("Checking for {attribute} mappings");

                let selector = Selector::parse(&format!(
                    "select[data-attribute_name='{attribute}'] > option"
                ))
                .unwrap();

                for attribute in html.select(&selector) {
                    let attr_key = element_extract_attr(attribute, "value")?;
                    let attr_name = element_to_text(attribute);

                    if !attr_key.is_empty() {
                        mapping.insert(attr_key, attr_name);
                    }
                }

                debug!("Setting {attribute} to {mapping:?}");

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
                warn!(
                    "Skipping product that is not present in API response: {variation_attr_key}:{variation_attr_value}"
                );

                return Ok(None);
            };

            attribute_names.push(attr_name.clone());
        }

        let flat_attr_names = attribute_names.join(" - ");

        Ok(Some(format!("{product_title} - {flat_attr_names}")))
    }
}

#[async_trait]
impl WooCommerceNested for WooCommerce {
    async fn parse_nested_products(
        url: String,
        category: Category,
        retailer_name: RetailerName,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut results: Vec<CrawlResult> = Vec::new();

        let request = RequestBuilder::new().set_url(&url).build();
        let result = WebClient::make_web_request(request).await?;

        let product_title = Self::get_nested_product_title(&result.body)?;

        let product_variations = Self::get_nested_product_variations(&result.body, &url)?;

        let attribute_mapping =
            Self::get_nested_product_attribute_name_mapping(&result.body, &product_variations)?;

        debug!("{attribute_mapping:?}");

        for variation in product_variations {
            if !variation.is_in_stock {
                debug!("Variant out of stock: {variation:?}");
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

            debug!("Pushed variant {name}");

            let new_result = CrawlResult::new(name, url.clone(), price, retailer_name, category)
                .with_image_url(variation.image.url);

            results.push(new_result);
        }

        sleep(Duration::from_secs(CRAWL_COOLDOWN_SECS)).await;

        Ok(results)
    }
}
