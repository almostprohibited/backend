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
use scraper::{ElementRef, Html, Selector};
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
const SITE_MAP: &str = "https://www.solelyoutdoors.com/sitemap.xml";
const PRODUCT_BASE_URL: &str = "https://www.solelyoutdoors.com/";
const URL: &str =
    "https://www.solelyoutdoors.com/{category}/page{page}.html?limit={page_limit}&sort=default";

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
        debug!("Received non f32 value");
        return Ok(Vec::new());
    };

    Ok(input_as_hashmap.into_values().collect())
}

#[derive(Debug)]
struct ProductPair {
    url: String,
    image_url: String,
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

pub struct SoleyOutdoors {
    search_queries: Vec<HtmlSearchQuery>,
}

impl Default for SoleyOutdoors {
    fn default() -> Self {
        Self::new()
    }
}

impl SoleyOutdoors {
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

        if api_price.price_old != 0.0 {
            price.sale_price = Some(price.regular_price);
            price.regular_price = price_to_cents(api_price.price_old.to_string())?;
        }

        Ok(price)
    }

    // logic copied from woocommerce parser
    fn get_image(&self, wrapper: ElementRef) -> Result<String, RetailerError> {
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

    async fn parse_links(
        &self,
        product_links: Vec<ProductPair>,
        search_term: &HtmlSearchQuery,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut results: Vec<CrawlResult> = Vec::new();

        for product in product_links {
            let request = RequestBuilder::new().set_url(product.url.clone()).build();
            let crawler = UnprotectedCrawler::make_web_request(request).await?;

            let parsed_product = serde_json::from_str::<ApiResponse>(&crawler.body)?.product;

            // wait 2 seconds instead of default 10 since
            // their robots.txt seems to be fine with 2
            // (not that I would have listened anyways)
            sleep(Duration::from_secs(2)).await;

            if !parsed_product.stock.available {
                continue;
            }

            let product_url = product.url.replace("?format=json", "");

            if parsed_product.variants.len() == 0 {
                let new_result = CrawlResult::new(
                    parsed_product.title,
                    product_url,
                    Self::get_price(parsed_product.price)?,
                    self.get_retailer_name(),
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
                    self.get_retailer_name(),
                    search_term.category,
                )
                .with_image_url(product.image_url.clone());

                results.push(new_result);
            }
        }

        Ok(results)
    }
}

impl HtmlRetailerSuper for SoleyOutdoors {}

#[async_trait]
impl Retailer for SoleyOutdoors {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::SoleyOutdoors
    }

    async fn init(&mut self) -> Result<(), RetailerError> {
        let queries = get_search_queries(SITE_MAP, PRODUCT_BASE_URL, |link| {
            if link.contains("firearms/barrels/") {
                return None;
            }

            if link.starts_with("opitcs-plus/") // listen, soley is the one that misspelled optics here
                || link.starts_with("reloading/")
                || link.starts_with("shooting-firearm-acessories/")
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
impl HtmlRetailer for SoleyOutdoors {
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
                Selector::parse("div.product-grid > div.product-block-holder").unwrap();
            html.select(&product_selector)
                .map(|element| element.html().clone())
                .collect::<Vec<_>>()
        };

        let mut product_links: Vec<ProductPair> = Vec::new();

        for html_doc in products {
            let product_inner = Html::parse_fragment(&html_doc);
            let product = product_inner.root_element();

            let wrapper = extract_element_from_element(product, "div")?;

            let Ok(data_link) = element_extract_attr(wrapper, "data-json") else {
                warn!("Found link with no product URL: {wrapper:?}");
                continue;
            };

            if !data_link.starts_with(PRODUCT_BASE_URL) {
                warn!("Link is not same as retailer: {data_link}");
                continue;
            }

            let image_link = self.get_image(wrapper)?;

            product_links.push(ProductPair {
                url: data_link.clone(),
                image_url: image_link,
            });
        }

        Ok(self.parse_links(product_links, search_term).await?)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        self.search_queries.clone()
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        let fragment = Html::parse_document(response);
        let page_number_selector =
            Selector::parse("div.paginate > ul > li:not(.active).number > a").unwrap();

        let mut page_links = fragment.select(&page_number_selector);

        let Some(last_page_element) = page_links.next_back() else {
            return Ok(0);
        };

        string_to_u64(element_to_text(last_page_element))
    }
}
