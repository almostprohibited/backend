use std::u64;

use async_trait::async_trait;
use common::result::{
    base::{CrawlResult, Price},
    enums::{Category, RetailerName},
};
use crawler::request::{Request, RequestBuilder};
use serde::Deserialize;
use tracing::debug;

use crate::{
    errors::RetailerError,
    structures::{HtmlRetailer, HtmlRetailerSuper, HtmlSearchQuery, Retailer},
    utils::conversions::price_to_cents,
};

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

// Limit set by Shopify
const PAGE_LIMIT: u64 = 250;
// Their pages are 1-indexed (they map page=0 === page=1)
const URL: &str =
    "https://intersurplus.com/collections/{category}/products.json?limit={page_limit}&page={page}";
const PRODUCT_URL: &str = "https://intersurplus.com/collections/{category}/products";
const DEFAULT_IMAGE: &str =
    "https://intersurplus.com/cdn/shopifycloud/storefront/assets/no-image-50-e6fb86f4_360x.gif";

pub struct InterSurplus;

impl InterSurplus {
    pub fn new() -> Self {
        Self {}
    }
}

impl HtmlRetailerSuper for InterSurplus {}

impl Retailer for InterSurplus {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::InterSurplus
    }
}

#[async_trait]
impl HtmlRetailer for InterSurplus {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &HtmlSearchQuery,
    ) -> Result<Request, RetailerError> {
        let url = URL
            .replace("{page_limit}", &PAGE_LIMIT.to_string())
            .replace("{category}", &search_term.term.to_string())
            .replace("{page}", &(page_num + 1).to_string());

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

        let api_response = serde_json::from_str::<ApiResponse>(response)?;

        for product in api_response.products {
            let image = match product.images.first() {
                Some(image_obj) => image_obj.src.clone(),
                None => DEFAULT_IMAGE.to_string(),
            };

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

                if let Some(regular_price) = variant.compare_at_price {
                    price.sale_price = Some(price.regular_price);
                    price.regular_price = price_to_cents(regular_price)?;
                };

                let url = format!(
                    "{}/{}",
                    PRODUCT_URL.replace("{category}", &search_term.term.to_string()),
                    product.handle.clone()
                );

                let new_result = CrawlResult::new(
                    title,
                    url,
                    price,
                    self.get_retailer_name(),
                    search_term.category,
                )
                .with_image_url(image.clone());

                results.push(new_result);
            }
        }

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        let mut terms = vec![
            HtmlSearchQuery {
                term: "all-firearms".into(),
                category: Category::Firearm,
            },
            HtmlSearchQuery {
                term: "ammunitions".into(),
                category: Category::Ammunition,
            },
        ];

        vec![
            "all-arms-accessories",
            "bayonets",
            "stripped-receiver",
            "combination-combo-barrels",
            "pistol-barrels",
            "riffle-barrels", // lol
            "shotgun-barrels",
            // TODO: this section does have a "parent"
            // so categories might change?
            "stock",
            "magazine",
            "m98-parts",
            "m96-parts",
            "husqvarna-1600-parts",
            // end TODO
            "reloading-components",
        ]
        .iter()
        .for_each(|term| {
            terms.push(HtmlSearchQuery {
                term: term.to_string(),
                category: Category::Other,
            });
        });

        terms
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        let products = serde_json::from_str::<ApiResponse>(response)?;

        if products.products.len() < 250 {
            return Ok(0);
        }

        Ok(u64::MAX)
    }
}
