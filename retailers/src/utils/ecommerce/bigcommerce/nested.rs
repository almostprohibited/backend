use std::time::Duration;

use common::{
    constants::CRAWL_COOLDOWN_SECS,
    result::{
        base::{CrawlResult, Price},
        enums::{Category, RetailerName},
    },
};
use crawler::{request::RequestBuilder, traits::HttpMethod, unprotected::UnprotectedCrawler};
use scraper::{ElementRef, Html, Selector};
use tokio::time::sleep;
use tracing::{debug, error, info};

use crate::{
    errors::RetailerError,
    utils::{
        conversions::price_to_cents,
        ecommerce::{
            BigCommerce,
            bigcommerce::structs::{
                FormValuePair, JavascriptJson, NestedApiResponse, NestedApiResponsePrice,
                NestedProduct, QueryParams,
            },
        },
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

pub(crate) trait BigCommerceNested {
    fn enqueue_nested_product_element(
        &mut self,
        element: ElementRef,
        category: Category,
    ) -> Result<(), RetailerError>;

    async fn parse_nested_products(
        &self,
        site_url: impl Into<String>,
        retailer_name: RetailerName,
    ) -> Result<Vec<CrawlResult>, RetailerError>;

    // TODO: refactor this
    // this is here because alflahertys behaves differently
    fn enqueue_nested_product(
        &mut self,
        name: impl Into<String>,
        fallback_image_url: impl Into<String>,
        product_url: impl Into<String>,
        category: Category,
    ) -> Result<(), RetailerError>;
}

impl BigCommerce {
    /// For nested pricing using the JSON API response
    fn get_price_from_object(api_response: NestedApiResponsePrice) -> Result<Price, RetailerError> {
        if api_response.without_tax.currency != "CAD" {
            let message = format!(
                "Invalid pricing, API returned non CAD pricing: {}",
                api_response.without_tax.currency
            );
            error!(message);
            return Err(RetailerError::ApiResponseInvalidShape(message));
        };

        let mut price = Price {
            regular_price: price_to_cents(api_response.without_tax.value.to_string())?,
            sale_price: None,
        };

        if let Some(non_sale_price) = api_response.non_sale_price_without_tax {
            price.sale_price = Some(price.regular_price);
            price.regular_price = price_to_cents(non_sale_price.value.to_string())?;
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
                Some((_, json)) => {
                    serde_json::from_str::<JavascriptJson>(json.trim_end_matches(";"))?
                }
                None => {
                    let message = "Unexpected JS, failed to split variable and value".to_string();

                    error!(message);

                    return Err(RetailerError::GeneralError(message));
                }
            };

            for in_stock_id in json_result.product_attributes.in_stock_attributes {
                attributes_in_stock.push(in_stock_id.to_string());
            }

            break;
        }

        Ok(attributes_in_stock)
    }

    fn get_product_id(html: &str) -> Result<String, RetailerError> {
        let parsed_html = Html::parse_document(html);
        let element = parsed_html.root_element();

        let parent_id_el = extract_element_from_element(element, "input[name=product_id]")?;
        let parent_id = element_extract_attr(parent_id_el, "value")?;

        Ok(parent_id)
    }

    fn get_pairs_radio_buttons(
        element: ElementRef,
        in_stock_attr: &[String],
    ) -> Result<Vec<FormValuePair>, RetailerError> {
        let mut attrs: Vec<FormValuePair> = Vec::new();

        let form_key_el = extract_element_from_element(element, "input.form-radio")?;
        let form_key = element_extract_attr(form_key_el, "name")?;

        let selector = Selector::parse("label.form-option").unwrap();

        for option in element.select(&selector) {
            let attr_id = element_extract_attr(option, "data-product-attribute-value")?;

            // account for empty list since alflahertys does things different
            if !in_stock_attr.contains(&attr_id) && !in_stock_attr.is_empty() {
                continue;
            }

            let span_el = extract_element_from_element(option, "span.form-option-variant")?;
            let attr_name =
                element_extract_attr(span_el, "title").unwrap_or_else(|_| element_to_text(span_el));

            attrs.push(FormValuePair {
                form_id: form_key.clone(),
                form_attr_id: attr_id,
                attr_name,
            });
        }

        Ok(attrs)
    }

    fn get_pairs_dropdown(
        element: ElementRef,
        in_stock_attr: &[String],
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

            // account for empty list since alflahertys does things different
            if !in_stock_attr.contains(&attr_id) && !in_stock_attr.is_empty() {
                continue;
            }

            attrs.push(FormValuePair {
                form_id: form_key.clone(),
                form_attr_id: attr_id,
                attr_name,
            });
        }

        Ok(attrs)
    }

    fn get_models(html: &str, cart_url: impl Into<String>) -> Result<QueryParams, RetailerError> {
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
            let form_value_pairs = match element_extract_attr(variant, "data-product-attribute")?
                .to_lowercase()
                .as_str()
            {
                "set-select" => Self::get_pairs_dropdown(variant, &in_stock_attr_ids)?,
                "set-rectangle" => Self::get_pairs_radio_buttons(variant, &in_stock_attr_ids)?,
                _ => vec![],
            };

            if !form_value_pairs.is_empty() {
                form_options.push(form_value_pairs);
            }
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

    fn get_nested_name(
        item_name: &String,
        variants: &Vec<FormValuePair>,
        category: Category,
    ) -> String {
        let combined_sub_names: String = variants
            .iter()
            .flat_map(|pair| {
                // special handling for round counts (add "rds" if the attr_name is something like "20")
                let name = match category == Category::Ammunition
                    && pair.attr_name.parse::<u64>().is_ok()
                {
                    true => format!(" - {}rds", pair.attr_name),
                    false => format!(" - {}", pair.attr_name),
                };

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
}

impl BigCommerceNested for BigCommerce {
    fn enqueue_nested_product_element(
        &mut self,
        element: ElementRef,
        category: Category,
    ) -> Result<(), RetailerError> {
        self.parse_queue.push(NestedProduct {
            name: Self::get_item_name(element)?,
            fallback_image_url: Self::get_image_url(element)?,
            category,
            product_url: Self::get_item_link(element)?,
        });

        Ok(())
    }

    // TODO: refactor this
    // this is here because alflahertys behaves differently
    fn enqueue_nested_product(
        &mut self,
        name: impl Into<String>,
        fallback_image_url: impl Into<String>,
        product_url: impl Into<String>,
        category: Category,
    ) -> Result<(), RetailerError> {
        self.parse_queue.push(NestedProduct {
            name: name.into(),
            fallback_image_url: fallback_image_url.into(),
            category,
            product_url: product_url.into(),
        });

        Ok(())
    }

    async fn parse_nested_products(
        &self,
        site_url: impl Into<String>,
        retailer_name: RetailerName,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut site_url = site_url.into();

        if site_url.ends_with("/") {
            site_url.pop();
        }

        let cart_url = format!("{site_url}/cart.php");

        let mut nested_results: Vec<CrawlResult> = Vec::new();

        for nested_product in &self.parse_queue {
            let request = RequestBuilder::new()
                .set_url(nested_product.product_url.clone())
                .build();
            let result = UnprotectedCrawler::make_web_request(request).await?;

            let product_id = Self::get_product_id(&result.body)?;
            let api_url = format!("{site_url}/remote/v1/product-attributes/{product_id}");

            let nested_variants = Self::get_models(&result.body, cart_url.clone())?;

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
                    .set_url(api_url.clone())
                    .set_method(HttpMethod::POST)
                    .set_headers(
                        [(
                            "Content-Type".into(),
                            "application/x-www-form-urlencoded".into(),
                        )]
                        .as_ref(),
                    )
                    .set_body(body)
                    .build();

                let result = UnprotectedCrawler::make_web_request(request).await?;
                let response = serde_json::from_str::<NestedApiResponse>(&result.body)?;

                if !response.data.instock {
                    info!("Skipping out of stock {combined_attrs}");
                    continue;
                }

                let price = Self::get_price_from_object(response.data.price)?;

                let name =
                    Self::get_nested_name(&nested_product.name, &variants, nested_product.category);

                let image = match response.data.image {
                    Some(image_object) => image_object.get_image(),
                    _ => nested_product.fallback_image_url.clone(),
                };

                let new_result = CrawlResult::new(
                    name,
                    nested_product.product_url.clone(),
                    price,
                    retailer_name,
                    nested_product.category,
                )
                .with_image_url(image);

                nested_results.push(new_result);

                sleep(Duration::from_secs(CRAWL_COOLDOWN_SECS)).await;
            }
        }

        Ok(nested_results)
    }
}
