use std::time::Duration;

use common::result::{
    base::{CrawlResult, Price},
    enums::{Category, RetailerName},
};
use crawler::{request::RequestBuilder, traits::HttpMethod, unprotected::UnprotectedCrawler};
use scraper::{ElementRef, Html, Selector};
use serde_json::Value;
use tokio::time::sleep;
use tracing::{debug, error, info};

use crate::{
    errors::RetailerError,
    pagination_client::CRAWL_COOLDOWN_SECS,
    utils::{
        conversions::price_to_cents,
        html::{element_extract_attr, element_to_text, extract_element_from_element},
        json::{json_get_array, json_get_object},
    },
};

const REPLACEMENT_PATTERN: &str = "{:size}";
const REPLACEMENT_SIZE: &str = "300w";

#[derive(Debug, Clone)]
struct FormValuePair {
    form_id: String,
    form_attr_id: String,
    attr_name: String,
}

#[derive(Debug)]
struct QueryParams {
    form_pairs: Vec<Vec<FormValuePair>>,
}

impl QueryParams {
    fn new() -> Self {
        Self {
            form_pairs: Vec::new(),
        }
    }

    fn apply(&mut self, form_pairs: Vec<FormValuePair>) {
        if self.form_pairs.len() == 0 {
            for pair in form_pairs {
                let mut new_vec: Vec<FormValuePair> = Vec::new();
                new_vec.push(pair);

                self.form_pairs.push(new_vec);
            }
        } else {
            for new_pair in form_pairs {
                for current_pairs in &mut self.form_pairs {
                    current_pairs.push(new_pair.clone());
                }
            }
        }
    }
}

pub(crate) struct BigCommerceNested {}

impl BigCommerceNested {
    /// For nested pricing using the JSON API response
    pub(crate) fn get_price_from_object(obj: &Value) -> Result<Price, RetailerError> {
        // "price": {
        //     "without_tax": {
        //         "formatted": "$3,476.00",
        //         "value": 3476,
        //         "currency": "CAD"
        //     },
        //     "tax_label": "Tax",
        //     "sale_price_without_tax": { <-- not included in non sales
        //         "formatted": "$3,476.00",
        //         "value": 3476,
        //         "currency": "CAD"
        //     },
        //     "non_sale_price_without_tax": { <-- not included in non sales
        //         "formatted": "$3,950.00",
        //         "value": 3950,
        //         "currency": "CAD"
        //     }
        // },
        let price_obj = json_get_object(&obj, "price".into())?;

        let main_price = json_get_object(price_obj, "without_tax".into())?;
        let main_price_value = json_get_object(main_price, "value".into())?;

        let currency = json_get_object(&main_price, "currency".into())?;
        if let Some(currency_string) = currency.as_str()
            && currency_string != "CAD"
        {
            let message = format!("Invalid pricing, API returned non CAD pricing: {currency:?}");
            error!(message);
            return Err(RetailerError::ApiResponseInvalidShape(message.into()));
        };

        let Some(price_str) = main_price_value.as_f64() else {
            let message = format!("Failed to convert {} into f64", main_price_value);
            error!(message);
            return Err(RetailerError::ApiResponseInvalidShape(message));
        };

        let mut price = Price {
            regular_price: price_to_cents(price_str.to_string())?,
            sale_price: None,
        };

        if let Ok(non_sale_price) = json_get_object(price_obj, "non_sale_price_without_tax".into())
        {
            price.sale_price = Some(price.regular_price);

            let regular_price = json_get_object(non_sale_price, "value".into())?;
            let Some(regular_price_str) = regular_price.as_f64() else {
                let message = format!("Failed to convert {} into f64", regular_price);
                error!(message);
                return Err(RetailerError::ApiResponseInvalidShape(message));
            };

            price.regular_price = price_to_cents(regular_price_str.to_string())?;
        };

        Ok(price)
    }

    // cursed method that parses JSON "manually" with serde
    fn get_in_stock_attributes(root_element: ElementRef) -> Result<Vec<String>, RetailerError> {
        let script_selector = Selector::parse("script[type='text/javascript']").unwrap();

        let mut attributes_in_stock: Vec<String> = Vec::new();

        // I could get the model IDs in a single go, but more reliable
        // to parse the JSON instead of checking if the option says
        // "out of stock"
        for script in root_element.select(&script_selector) {
            let javascript = element_to_text(script);

            if !javascript.contains("in_stock_attributes") {
                debug!("JSON missing 'in_stock_attributes', skipping");
                continue;
            }

            let json_result = match javascript.split_once(" = ") {
                Some((_, json)) => serde_json::from_str::<Value>(json.trim_end_matches(";"))?,
                None => {
                    let message = format!("Unexpected JS, failed to split variable and value");

                    error!(message);

                    return Err(RetailerError::GeneralError(message));
                }
            };

            let attributes = json_get_object(&json_result, "product_attributes".into())?;
            let in_stock_prop = json_get_object(&attributes, "in_stock_attributes".into())?;
            let in_stock_array = json_get_array(&in_stock_prop)?;

            for in_stock_id in in_stock_array {
                let Some(id) = in_stock_id.as_u64() else {
                    let message = format!("In stock attribute is not a number");

                    error!(message);

                    return Err(RetailerError::GeneralError(message));
                };

                attributes_in_stock.push(id.to_string());
            }

            break;
        }

        Ok(attributes_in_stock)
    }

    fn get_product_id(html: &String) -> Result<String, RetailerError> {
        let parsed_html = Html::parse_document(html);
        let element = parsed_html.root_element();

        let parent_id_el = extract_element_from_element(element, "input[name=product_id]")?;
        let parent_id = element_extract_attr(parent_id_el, "value")?;

        Ok(parent_id)
    }

    fn get_pairs_radio_buttons(
        element: ElementRef,
        in_stock_attr: &Vec<String>,
    ) -> Result<Vec<FormValuePair>, RetailerError> {
        let mut attrs: Vec<FormValuePair> = Vec::new();

        let form_key_el = extract_element_from_element(element, "input.form-radio")?;
        let form_key = element_extract_attr(form_key_el, "name")?;

        let selector = Selector::parse("label.form-option").unwrap();

        for option in element.select(&selector) {
            let attr_id = element_extract_attr(option, "data-product-attribute-value")?;

            if !in_stock_attr.contains(&attr_id) {
                continue;
            }

            let span_el = extract_element_from_element(option, "span.form-option-variant")?;
            let attr_name =
                element_extract_attr(span_el, "title").unwrap_or_else(|_| element_to_text(span_el));

            attrs.push(FormValuePair {
                form_id: form_key.clone(),
                form_attr_id: attr_id,
                attr_name: attr_name,
            });
        }

        Ok(attrs)
    }

    fn get_pairs_dropdown(
        element: ElementRef,
        in_stock_attr: &Vec<String>,
    ) -> Result<Vec<FormValuePair>, RetailerError> {
        let mut attrs: Vec<FormValuePair> = Vec::new();

        let form_key_id =
            extract_element_from_element(element, "select.form-select.form-select--small")?;
        let form_key = element_extract_attr(form_key_id, "name")?;

        let selector =
            Selector::parse("select.form-select--small > option[data-product-attribute-value]")
                .unwrap();

        for option in element.select(&selector) {
            let attr_name = element_to_text(option);
            let attr_id = element_extract_attr(option, "data-product-attribute-value")?;

            if !in_stock_attr.contains(&attr_id) {
                continue;
            }

            attrs.push(FormValuePair {
                form_id: form_key.clone(),
                form_attr_id: attr_id,
                attr_name: attr_name,
            });
        }

        Ok(attrs)
    }

    fn get_models(
        html: &String,
        cart_url: impl Into<String>,
    ) -> Result<QueryParams, RetailerError> {
        let parsed_html = Html::parse_document(html);
        let element = parsed_html.root_element();

        let in_stock_attr_ids = Self::get_in_stock_attributes(element)?;
        debug!("Variants in stock: {:?}", in_stock_attr_ids);

        let selector = format!(
            "form[action='{}'] div.form-field[data-product-attribute]",
            cart_url.into()
        );
        let variant_selector = Selector::parse(&selector).unwrap();

        /*
           [
               [form1-option1, form1-option2],
               [form2-option1, form2-option2]
           ]
        */
        let mut form_options: Vec<Vec<FormValuePair>> = Vec::new();

        for variant in element.select(&variant_selector) {
            let form_value_pairs = match extract_element_from_element(
                variant,
                "select.form-select.form-select--small",
            ) {
                Ok(_) => Self::get_pairs_dropdown(variant, &in_stock_attr_ids)?,
                Err(_) => Self::get_pairs_radio_buttons(variant, &in_stock_attr_ids)?,
            };

            form_options.push(form_value_pairs);
        }

        /*
           [
                (form1-option1, form2-option1),
                (form1-option1, form2-option2),
                (form1-option2, form2-option1),
                (form1-option2, form2-option2)
           ]
        */
        let mut query_params: QueryParams = QueryParams::new();

        for option in form_options {
            query_params.apply(option);
        }

        Ok(query_params)
    }

    fn get_name(item_name: &String, variants: &Vec<FormValuePair>) -> String {
        let combined_sub_names: String = variants
            .iter()
            .flat_map(|pair| {
                let name = format!(" - {}", pair.attr_name);
                name.chars().collect::<Vec<_>>()
            })
            .collect();

        let name = format!("{item_name}{combined_sub_names}");

        debug!(
            "Transforming {:?} to '{}' for final name: {}",
            variants, combined_sub_names, name
        );

        name
    }

    fn get_image_url(obj: &Value) -> Result<String, RetailerError> {
        let image_obj = json_get_object(&obj, "image".into())?;
        let image_url_value = json_get_object(&image_obj, "data".into())?;

        let Some(image_url) = image_url_value.as_str() else {
            return Err(RetailerError::ApiResponseInvalidShape(
                "Expected img.data to be a string".into(),
            ));
        };

        Ok(image_url.replace(REPLACEMENT_PATTERN, REPLACEMENT_SIZE))
    }

    pub(crate) async fn parse_nested(
        api_url: impl Into<String>,
        item_url: impl Into<String>,
        retailer_cart_url: impl Into<String>,
        retailer: RetailerName,
        category: Category,
        item_name: String,
        fallback_image_url: String,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let crawler = UnprotectedCrawler::new();

        let api_url_string: String = api_url.into();
        let item_url_string: String = item_url.into();

        let mut nested_results: Vec<CrawlResult> = Vec::new();

        let request = RequestBuilder::new()
            .set_url(item_url_string.clone())
            .build();
        let result = crawler.make_web_request(request).await?;

        let product_id = Self::get_product_id(&result.body)?;
        let nested_variants = Self::get_models(&result.body, retailer_cart_url.into())?;

        for variants in nested_variants.form_pairs {
            let combined_attrs: String = variants
                .iter()
                .flat_map(|pair| {
                    let attr = format!("&{}={}", pair.form_id, pair.form_attr_id);
                    attr.chars().collect::<Vec<_>>()
                })
                .collect();
            let body = format!("action=add&product_id={product_id}{combined_attrs}");

            debug!("Sending subrequest with {}", body);

            let request = RequestBuilder::new()
                .set_url(api_url_string.replace("{product_id}", &product_id))
                .set_method(HttpMethod::POST)
                .set_headers(
                    &[(
                        "Content-Type".into(),
                        "application/x-www-form-urlencoded".into(),
                    )]
                    .to_vec(),
                )
                .set_body(body)
                .build();

            let result = crawler.make_web_request(request).await?;

            let json = serde_json::from_str::<Value>(&result.body)?;
            let data = json_get_object(&json, "data".into())?;

            if json_get_object(&data, "instock".into())? == false {
                info!("Skipping out of stock {combined_attrs}");
                continue;
            }

            let price = Self::get_price_from_object(data)?;

            let name = Self::get_name(&item_name, &variants);
            let image = Self::get_image_url(&data).unwrap_or(fallback_image_url.clone());

            let new_result =
                CrawlResult::new(name, item_url_string.clone(), price, retailer, category)
                    .with_image_url(image);

            nested_results.push(new_result);

            sleep(Duration::from_secs(CRAWL_COOLDOWN_SECS)).await;
        }

        Ok(nested_results)
    }
}
