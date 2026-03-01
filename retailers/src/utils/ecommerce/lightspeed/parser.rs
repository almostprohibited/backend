use std::{collections::HashMap, time::Duration};

use common::result::{
    base::{CrawlResult, Price},
    enums::RetailerName,
};
use crawler::{request::RequestBuilder, unprotected::UnprotectedCrawler};
use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Deserializer};
use tokio::time::sleep;
use tracing::{debug, error, warn};

use crate::{
    errors::RetailerError,
    structures::HtmlSearchQuery,
    utils::{
        conversions::{price_to_cents, string_to_u64},
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

#[derive(Debug)]
struct ProductPair {
    url: String,
    image_url: String,
}

// convert [hashmap | bool] into vec of variants
fn variants_boolean_to_variants<'de, D>(
    deserializer: D,
) -> Result<Vec<ApiResponseVariant>, D::Error>
where
    D: Deserializer<'de>,
{
    let input: Result<HashMap<String, ApiResponseVariant>, D::Error> =
        HashMap::deserialize(deserializer);

    let Ok(input_as_hashmap) = input else {
        debug!("Received non hashmap value");
        return Ok(Vec::new());
    };

    Ok(input_as_hashmap.into_values().collect())
}

#[derive(Deserialize)]
struct ApiResponse {
    product: ApiResponseProduct,
}

#[derive(Deserialize)]
struct ApiResponseProduct {
    title: String,
    stock: ApiResponseStock,
    price: ApiResponsePrice,
    #[serde(deserialize_with = "variants_boolean_to_variants")]
    variants: Vec<ApiResponseVariant>,
}

#[derive(Deserialize)]
struct ApiResponseStock {
    available: bool,
}

#[derive(Deserialize)]
struct ApiResponsePrice {
    price: f32,
    price_old: f32,
}

#[derive(Deserialize)]
struct ApiResponseVariant {
    title: String,
    stock: ApiResponseStock,
    price: ApiResponsePrice,
}

pub(crate) struct LightSpeed;

impl LightSpeed {
    pub(crate) fn get_max_pages(html: &str, selector: &str) -> Result<u64, RetailerError> {
        let fragment = Html::parse_document(html);
        let page_number_selector = Selector::parse(selector).unwrap();

        let mut page_links = fragment.select(&page_number_selector);

        let Some(last_page_element) = page_links.next_back() else {
            return Ok(0);
        };

        string_to_u64(element_to_text(last_page_element))
    }

    pub(crate) async fn parse_products(
        base_url: &str,
        selector: &str,
        response: &str,
        search_term: &HtmlSearchQuery,
        retailer: RetailerName,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let links = Self::extract_links(base_url, response, selector)?;

        Self::visit_pages(links, search_term, retailer).await
    }

    fn get_price(api_price: ApiResponsePrice) -> Result<Price, RetailerError> {
        let mut price = Price {
            regular_price: price_to_cents(api_price.price.to_string())?,
            sale_price: None,
        };

        if api_price.price_old != 0.0 {
            price.sale_price = Some(price.regular_price);
            price.regular_price = price_to_cents(api_price.price_old.to_string())?;
        }

        Ok(price)
    }

    // logic copied from woocommerce parser
    fn get_image(wrapper: ElementRef) -> Result<String, RetailerError> {
        let image_element =
            extract_element_from_element(wrapper, "div.product-block-image > a > img")?;

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

    fn extract_links(
        base_url: &str,
        response: &str,
        selector: &str,
    ) -> Result<Vec<ProductPair>, RetailerError> {
        let html = Html::parse_document(response);
        let product_selector = Selector::parse(selector).unwrap();

        let mut product_links: Vec<ProductPair> = Vec::new();

        for product in html.select(&product_selector) {
            let Ok(data_link) = element_extract_attr(product, "data-json") else {
                warn!("Found link with no product URL: {product:?}");
                continue;
            };

            if !data_link.starts_with(base_url) {
                warn!("Link is not same as retailer: {data_link}");
                continue;
            }

            let image_link = Self::get_image(product)?;

            product_links.push(ProductPair {
                url: data_link.clone(),
                image_url: image_link,
            });
        }

        Ok(product_links)
    }

    async fn visit_pages(
        links: Vec<ProductPair>,
        search_term: &HtmlSearchQuery,
        retailer: RetailerName,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut results: Vec<CrawlResult> = Vec::new();

        for product in links {
            let request = RequestBuilder::new().set_url(product.url.clone()).build();
            let crawler = UnprotectedCrawler::make_web_request(request).await?;

            sleep(Duration::from_secs(2)).await;

            // TODO: do something about reporting this as a partial error
            // https://github.com/almostprohibited/backend/issues/4
            //
            // this logic is here for Cabin Creek, they have a product that
            // does not load properly
            let parsed_product = match serde_json::from_str::<ApiResponse>(&crawler.body) {
                Ok(response) => response.product,
                Err(_) => {
                    error!("Failed to deserialize response for {}", product.url);

                    continue;
                }
            };

            if !parsed_product.stock.available {
                continue;
            }

            let product_url = product.url.replace("?format=json", "");

            if parsed_product.variants.len() == 0 {
                let new_result = CrawlResult::new(
                    parsed_product.title,
                    product_url,
                    Self::get_price(parsed_product.price)?,
                    retailer,
                    search_term.category,
                )
                .with_image_url(product.image_url);

                results.push(new_result);

                continue;
            }

            for nested_product in parsed_product.variants {
                if !nested_product.stock.available {
                    continue;
                }

                let new_result = CrawlResult::new(
                    format!("{} - {}", parsed_product.title, nested_product.title),
                    product_url.clone(),
                    Self::get_price(nested_product.price)?,
                    retailer,
                    search_term.category,
                )
                .with_image_url(product.image_url.clone());

                results.push(new_result);
            }
        }

        Ok(results)
    }
}
