use common::result::{
    base::{CrawlResult, Price},
    enums::RetailerName,
};
use serde::Deserialize;

use crate::{
    errors::RetailerError, structures::HtmlSearchQuery, utils::conversions::price_to_cents,
};

// Limit set by Shopify
pub(crate) const PAGE_LIMIT: u64 = 250;

#[derive(Deserialize)]
struct ApiResponse {
    products: Vec<Product>,
}

#[derive(Deserialize)]
struct Product {
    title: String,
    handle: String,
    variants: Vec<Variant>,
    // I hope the images are in order, they are in the test response
    images: Vec<Image>,
}

#[derive(Deserialize)]
struct Variant {
    title: String,
    available: bool,
    price: String,
    compare_at_price: Option<String>,
}

#[derive(Deserialize)]
struct Image {
    src: String,
}

pub(crate) struct Shopify {
    default_image: String,
    retailer: RetailerName,
    base_url: String,
}

impl Shopify {
    pub(crate) fn new(default_image: &str, retailer: RetailerName, base_url: &str) -> Self {
        let mut url = base_url.to_string();

        if url.ends_with("/") {
            url.pop();
        }

        Self {
            default_image: default_image.to_string(),
            retailer,
            base_url: url,
        }
    }

    pub(crate) fn parse_response(
        &self,
        response: &String,
        search_term: &HtmlSearchQuery,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut results: Vec<CrawlResult> = Vec::new();

        let api_response = serde_json::from_str::<ApiResponse>(response)?;

        for product in api_response.products {
            let mut image = match product.images.first() {
                Some(image_obj) => image_obj.src.clone(),
                None => self.default_image.clone(),
            };

            if let Some(index) = image.find("?v=") {
                let _ = image.split_off(index);
            }

            for variant in product.variants {
                if !variant.available {
                    continue;
                }

                let mut title = product.title.clone();

                // here's hoping that this NEVER changes
                // otherwise I'm going to end up with a ton of random text
                if variant.title.to_lowercase() != "default title" {
                    title = format!("{title} - {}", variant.title);
                }

                let mut price = Price {
                    regular_price: price_to_cents(variant.price)?,
                    sale_price: None,
                };

                if let Some(compare_at_price) = variant.compare_at_price {
                    let compare_at_price_cents = price_to_cents(compare_at_price)?;

                    if compare_at_price_cents > price.regular_price {
                        price.sale_price = Some(price.regular_price);
                        price.regular_price = compare_at_price_cents;
                    }
                };

                let url = format!(
                    "{}/collections/{}/products/{}",
                    self.base_url, search_term.term, product.handle
                );

                let new_result =
                    CrawlResult::new(title, url, price, self.retailer, search_term.category)
                        .with_image_url(image.clone());

                results.push(new_result);
            }
        }

        Ok(results)
    }

    pub(crate) fn get_pages(response: &String) -> Result<u64, RetailerError> {
        // yes, this deserialize just to get the page number is wasteful
        let products = serde_json::from_str::<ApiResponse>(response)?;

        if products.products.len() < 250 {
            return Ok(0);
        }

        Ok(u64::MAX)
    }
}
