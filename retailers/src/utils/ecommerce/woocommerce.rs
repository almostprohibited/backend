use std::{collections::HashMap, time::Duration};

use common::{
    constants::CRAWL_COOLDOWN_SECS,
    result::{
        base::{CrawlResult, Price},
        enums::{Category, RetailerName},
    },
};
use crawler::{request::RequestBuilder, unprotected::UnprotectedCrawler};
use scraper::{ElementRef, Html, Selector};
use serde::Deserialize;
use tokio::time::sleep;

use crate::{
    errors::RetailerError,
    utils::{
        conversions::{price_to_cents, string_to_u64},
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

pub(crate) struct WooCommerceBuilder {
    product_name_selector: String,
    product_url_selector: String,
    image_url_selector: String,
}

impl WooCommerceBuilder {
    pub(crate) fn default() -> Self {
        Self {
            product_name_selector: "div.product-element-bottom > h3 > a".into(),
            product_url_selector: "div.product-element-bottom > h3 > a".into(),
            image_url_selector: "a.product-image-link > img".into(),
        }
    }

    pub(crate) fn with_product_name_selector(mut self, selector: impl Into<String>) -> Self {
        self.product_name_selector = selector.into();

        self
    }

    pub(crate) fn with_product_url_selector(mut self, selector: impl Into<String>) -> Self {
        self.product_url_selector = selector.into();

        self
    }

    pub(crate) fn with_image_url_selector(mut self, selector: impl Into<String>) -> Self {
        self.image_url_selector = selector.into();

        self
    }

    pub(crate) fn build(self) -> WooCommerce {
        WooCommerce {
            options: self,
            nested_queue: Vec::new(),
        }
    }
}

pub(crate) struct WooCommerce {
    options: WooCommerceBuilder,
    nested_queue: Vec<NestedProduct>,
}

impl WooCommerce {
    fn parse_price(element: ElementRef) -> Result<Price, RetailerError> {
        let mut price = Price {
            regular_price: 0,
            sale_price: None,
        };

        let regular_non_sale_price =
            extract_element_from_element(element, "span.price > span.amount > bdi");

        match regular_non_sale_price {
            Ok(regular_price_element) => {
                price.regular_price = price_to_cents(element_to_text(regular_price_element))?;
            }
            Err(_) => {
                let sale_price =
                    extract_element_from_element(element, "span.price > ins > span.amount > bdi")?;
                let previous_price =
                    extract_element_from_element(element, "span.price > del > span.amount > bdi")?;

                price.regular_price = price_to_cents(element_to_text(previous_price))?;
                price.sale_price = Some(price_to_cents(element_to_text(sale_price))?);
            }
        }

        Ok(price)
    }

    pub(crate) fn parse_max_pages(response: &str) -> Result<u64, RetailerError> {
        let fragment = Html::parse_document(response);
        let page_number_selector =
            Selector::parse("ul.page-numbers > li > a:not(.next):not(.prev).page-numbers").unwrap();

        let mut page_links = fragment.select(&page_number_selector);

        let Some(last_page_element) = page_links.next_back() else {
            return Ok(0);
        };

        string_to_u64(element_to_text(last_page_element))
    }

    fn get_image_url(&self, element: ElementRef) -> Result<String, RetailerError> {
        let image_element =
            extract_element_from_element(element, self.options.image_url_selector.clone())?;

        if let Ok(data_src) = element_extract_attr(image_element, "data-src")
            && data_src.starts_with("https")
            && !data_src.contains("lazy")
        {
            return Ok(data_src);
        };

        if let Ok(regular_src) = element_extract_attr(image_element, "src")
            && regular_src.starts_with("https")
            && !regular_src.contains("lazy")
        {
            return Ok(regular_src);
        }

        Err(RetailerError::HtmlElementMissingAttribute(
            "'valid data-src or src'".into(),
            element_to_text(image_element),
        ))
    }

    pub(crate) fn parse_product(
        &self,
        element: ElementRef,
        retailer: RetailerName,
        category: Category,
    ) -> Result<CrawlResult, RetailerError> {
        let url_element =
            extract_element_from_element(element, self.options.product_url_selector.clone())?;
        let name_element =
            extract_element_from_element(element, self.options.product_name_selector.clone())?;

        let name = element_to_text(name_element);
        let url = element_extract_attr(url_element, "href")?;

        let image_url = self.get_image_url(element)?;

        let new_product =
            CrawlResult::new(name, url, Self::parse_price(element)?, retailer, category)
                .with_image_url(image_url);

        Ok(new_product)
    }

    pub(crate) fn enqueue_nested_product(&mut self, url: String, category: Category) {
        self.nested_queue.push(NestedProduct { url, category });
    }

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

        Ok(serde_json::from_str::<Vec<ProductVariation>>(
            &form_attribute,
        )?)
    }

    fn get_nested_product_title(result: &String) -> Result<String, RetailerError> {
        let html = Html::parse_document(result);
        let title = extract_element_from_element(html.root_element(), "h1.product_title")?;

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

    pub(crate) async fn parse_nested_products(
        &self,
        retailer_name: RetailerName,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut results: Vec<CrawlResult> = Vec::new();

        for nested_product in &self.nested_queue {
            let request = RequestBuilder::new().set_url(&nested_product.url).build();
            let result = UnprotectedCrawler::make_web_request(request).await?;

            let product_title = Self::get_nested_product_title(&result.body)?;

            let product_variations =
                Self::get_nested_product_variations(&result.body, &nested_product.url)?;

            let attribute_mapping =
                Self::get_nested_product_attribute_name_mapping(&result.body, &product_variations)?;

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
                    retailer_name,
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
