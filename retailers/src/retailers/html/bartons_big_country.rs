use std::{collections::HashMap, time::Duration};

use async_trait::async_trait;
use common::result::{
    base::{CrawlResult, Price},
    enums::{Category, RetailerName},
};
use crawler::{
    request::{Request, RequestBuilder},
    unprotected::UnprotectedCrawler,
};
use scraper::{Html, Selector};
use serde::{Deserialize, Deserializer};
use tokio::time::sleep;
use tracing::{debug, warn};

use crate::{
    errors::RetailerError,
    structures::{HtmlRetailer, HtmlRetailerSuper, HtmlSearchQuery, Retailer},
    utils::{
        conversions::{price_to_cents, string_to_u64},
        generic_sitemap::get_search_queries,
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

const PAGE_LIMIT: u64 = 100;
const SITE_MAP: &str = "https://www.bartonsbigcountry.ca/sitemap.xml";
const PRODUCT_BASE_URL: &str = "https://www.bartonsbigcountry.ca/";
const URL: &str =
    "https://www.bartonsbigcountry.ca/{category}/page{page}.html?limit={page_limit}&sort=default";

#[derive(Deserialize)]
#[serde(untagged)]
enum IntermediateInput {
    Map(HashMap<String, ApiResponseVariant>),
    Vec(Vec<ApiResponseVariant>),
}

// convert [hashmap | vec] to vec
fn hashmap_vec_to_vec<'de, D>(deserializer: D) -> Result<Vec<ApiResponseVariant>, D::Error>
where
    D: Deserializer<'de>,
{
    let input = IntermediateInput::deserialize(deserializer)?;

    let result = match input {
        IntermediateInput::Map(hash_map) => hash_map.into_values().collect(),
        IntermediateInput::Vec(api_response_variants) => api_response_variants,
    };

    Ok(result)
}

// convert [f32 | bool] into Option<f32>
fn f32_boolean_to_f32_option<'de, D>(deserializer: D) -> Result<Option<f32>, D::Error>
where
    D: Deserializer<'de>,
{
    let input: Result<f32, D::Error> = f32::deserialize(deserializer);

    let Ok(input_as_f32) = input else {
        debug!("Received non f32 value");
        return Ok(None);
    };

    Ok(Some(input_as_f32))
}

#[derive(Deserialize)]
struct ApiResponse {
    fulltitle: String,
    stock: ApiResponseStock,
    #[serde(deserialize_with = "hashmap_vec_to_vec")]
    variants: Vec<ApiResponseVariant>, // why, this is either an empty list if no variants, or a hashmap
    price: ApiResponsePrice,
    image: String,
    image_id: u32,
}

#[derive(Deserialize)]
struct ApiResponseStock {
    available: bool,
}

#[derive(Deserialize)]
struct ApiResponsePrice {
    price: f32,
    #[serde(deserialize_with = "f32_boolean_to_f32_option")]
    price_old: Option<f32>, // annoyingly, their API returns [f32 | boolean]
}

#[derive(Deserialize)]
struct ApiResponseVariant {
    price: ApiResponsePrice,
    url: String,
    title: String,
    image: Option<u32>,
    stock: ApiResponseStock,
}

pub struct BartonsBigCountry {
    search_queries: Vec<HtmlSearchQuery>,
}

impl Default for BartonsBigCountry {
    fn default() -> Self {
        Self::new()
    }
}

impl BartonsBigCountry {
    pub fn new() -> Self {
        Self {
            search_queries: Vec::new(),
        }
    }

    fn get_price(api_price: ApiResponsePrice) -> Result<Price, RetailerError> {
        let mut price = Price {
            regular_price: price_to_cents(api_price.price.to_string())?,
            sale_price: None,
        };

        if let Some(old_price) = api_price.price_old {
            price.sale_price = Some(price.regular_price);
            price.regular_price = price_to_cents(old_price.to_string())?;
        }

        Ok(price)
    }

    async fn parse_links(
        &self,
        product_links: Vec<String>,
        search_term: &HtmlSearchQuery,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut results: Vec<CrawlResult> = Vec::new();

        for product_url in product_links {
            let request = RequestBuilder::new()
                .set_url(product_url.replace(".html", ".ajax"))
                .build();
            let crawler = UnprotectedCrawler::make_web_request(request).await?;

            let parsed_product = serde_json::from_str::<ApiResponse>(&crawler.body)?;

            if !parsed_product.stock.available {
                continue;
            }

            sleep(Duration::from_secs(2)).await;

            if parsed_product.variants.len() == 0 {
                let price = Self::get_price(parsed_product.price)?;

                let new_result = CrawlResult::new(
                    parsed_product.fulltitle,
                    product_url,
                    price,
                    self.get_retailer_name(),
                    search_term.category,
                )
                .with_image_url(parsed_product.image);

                results.push(new_result);

                continue;
            }

            for nested_product in parsed_product.variants {
                if !nested_product.stock.available {
                    continue;
                }

                let price = Self::get_price(nested_product.price)?;

                let mut product_name = parsed_product.fulltitle.clone();

                if let Some((_, variant_name)) = nested_product.title.split_once(" : ") {
                    product_name = format!("{product_name} - {variant_name}");
                }

                let mut new_result = CrawlResult::new(
                    product_name,
                    nested_product.url,
                    price,
                    self.get_retailer_name(),
                    search_term.category,
                );

                let mut image_url = parsed_product.image.replace("/50x50", "/512x512");

                if let Some(nested_image_id) = nested_product.image {
                    image_url = image_url.replace(
                        &parsed_product.image_id.to_string(),
                        &nested_image_id.to_string(),
                    );
                }

                new_result = new_result.with_image_url(image_url);

                results.push(new_result);
            }
        }

        Ok(results)
    }
}

impl HtmlRetailerSuper for BartonsBigCountry {}

#[async_trait]
impl Retailer for BartonsBigCountry {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::BartonsBigCountry
    }

    async fn init(&mut self) -> Result<(), RetailerError> {
        let queries = get_search_queries(SITE_MAP, PRODUCT_BASE_URL, |link| {
            if link.contains("firearms/crossbow") {
                return None;
            }

            if link.starts_with("firearm-accessories/")
                || link.starts_with("lights/firearm-mountable")
                || link.starts_with("optics/")
                || link.starts_with("reloading/")
                || link.starts_with("shooting/")
                || link.starts_with("tactical/")
            {
                return Some(HtmlSearchQuery {
                    term: link,
                    category: Category::Other,
                });
            } else if link.starts_with("ammunition/") {
                return Some(HtmlSearchQuery {
                    term: link,
                    category: Category::Ammunition,
                });
            } else if link.starts_with("firearms/") {
                return Some(HtmlSearchQuery {
                    term: link,
                    category: Category::Firearm,
                });
            };

            None
        })
        .await?;

        self.search_queries.extend(queries);

        Ok(())
    }
}

#[async_trait]
impl HtmlRetailer for BartonsBigCountry {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &HtmlSearchQuery,
    ) -> Result<Request, RetailerError> {
        let url = URL
            .replace("{category}", &search_term.term)
            .replace("{page_limit}", &PAGE_LIMIT.to_string())
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
        let products = {
            let html = Html::parse_document(response);
            let product_selector =
                Selector::parse("div#products-container > div.productborder div.image-wrap")
                    .unwrap();
            html.select(&product_selector)
                .map(|element| element.html().clone())
                .collect::<Vec<_>>()
        };

        let mut product_links: Vec<String> = Vec::new();

        for html_doc in products {
            let product_inner = Html::parse_fragment(&html_doc);
            let product = product_inner.root_element();

            let wrapper = extract_element_from_element(product, "a")?;

            let link = element_extract_attr(wrapper, "href")?;

            if !link.starts_with(PRODUCT_BASE_URL) {
                warn!("Link is not same as retailer: {link}");
                continue;
            }

            product_links.push(link);
        }

        Ok(self.parse_links(product_links, search_term).await?)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        self.search_queries.clone()
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        let fragment = Html::parse_document(response);
        let page_number_selector =
            Selector::parse("div.pager > ul > li:not(.active).number > a").unwrap();

        let mut page_links = fragment.select(&page_number_selector);

        let Some(last_page_element) = page_links.next_back() else {
            return Ok(0);
        };

        string_to_u64(element_to_text(last_page_element))
    }
}
