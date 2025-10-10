use async_trait::async_trait;
use common::result::{base::CrawlResult, enums::RetailerName};
use crawler::{
    request::{Request, RequestBuilder},
    traits::HttpMethod,
    unprotected::UnprotectedCrawler,
};
use regex::Regex;
use serde_json::json;
use tracing::warn;

use crate::{
    errors::RetailerError,
    retailers::gql::prophet_river::{
        api_request::{API_QUERY_REQUEST, PAGINATION_REPLACEMENT_KEY},
        api_response_objects::ApiResponse,
    },
    structures::{GqlRetailer, GqlRetailerSuper, Retailer},
    utils::regex::unwrap_regex_capture,
};

const DEFAULT_IMAGE_URL: &str = "https://cdn11.bigcommerce.com/s-dcynby20nc/stencil/be1fd970-0d6b-013e-f9b9-6613132a0701/e/092afc30-45f5-013e-ca76-52b5c4b168da/img/ProductDefault.gif";
const MAIN_URL: &str = "https://store.prophetriver.com";
const GQL_URL: &str = "https://store.prophetriver.com/graphql";

pub struct ProphetRiver {
    auth_token: String,
}

impl Default for ProphetRiver {
    fn default() -> Self {
        Self::new()
    }
}

impl ProphetRiver {
    pub fn new() -> Self {
        Self {
            auth_token: String::new(),
        }
    }

    async fn get_auth_token() -> Result<String, RetailerError> {
        let crawler = UnprotectedCrawler::new();
        let request = RequestBuilder::new().set_url(MAIN_URL).build();

        let response = crawler.make_web_request(request).await?.body;

        let regex = Regex::new(
            r"'Authorization'\s*:\s*'Bearer\s+([A-Za-z0-9-_]+\.[A-Za-z0-9-_]+\.[A-Za-z0-9-_]+)'",
        )
        .expect("Prophet River regex to not fail creation");

        let token = unwrap_regex_capture(&regex, &response)?;

        Ok(token)
    }
}

impl GqlRetailerSuper for ProphetRiver {}

#[async_trait]
impl Retailer for ProphetRiver {
    async fn init(&mut self) -> Result<(), RetailerError> {
        self.auth_token = Self::get_auth_token().await?;

        Ok(())
    }

    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::ProphetRiver
    }
}

#[async_trait]
impl GqlRetailer for ProphetRiver {
    async fn build_page_request(
        &self,
        pagination_token: Option<String>,
    ) -> Result<Request, RetailerError> {
        let mut pagination_entry = String::new();

        if let Some(token) = pagination_token {
            pagination_entry = format!("after: \"{token}\"");
        };

        let request_json = json!({
            "query": API_QUERY_REQUEST.replace(PAGINATION_REPLACEMENT_KEY, &pagination_entry)
        });

        let authorization_header = format!("Bearer {}", self.auth_token);

        let request = RequestBuilder::new()
            .set_url(GQL_URL)
            .set_method(HttpMethod::POST)
            .set_headers(
                &[
                    ("Content-Type".into(), "application/json".into()),
                    ("Authorization".into(), authorization_header),
                ]
                .to_vec(),
            )
            .set_json_body(request_json)
            .build();

        Ok(request)
    }

    async fn parse_response(&self, response: &str) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut results: Vec<CrawlResult> = Vec::new();

        let response_objects = serde_json::from_str::<ApiResponse>(response)?;

        for edge in response_objects.data.site.products.edges {
            let node = edge.node;

            if !node.inventory.is_in_stock {
                continue;
            }

            if node.inventory.has_variant_inventory {
                return Err(RetailerError::GeneralError(format!(
                    "Failed to parse object {} since it contains variants",
                    node.name
                )));
            }

            let Some(category) = node.categories.get_category() else {
                warn!(
                    "Skipping unrecognized item: {} (listed under {:?})",
                    node.name, node.categories
                );
                continue;
            };

            let url = format!("{MAIN_URL}{}", node.path);

            let image_url = match node.default_image {
                Some(api_image) => api_image.url,
                None => DEFAULT_IMAGE_URL.into(),
            };

            let new_result = CrawlResult::new(
                node.name,
                url,
                node.prices.get_price()?,
                self.get_retailer_name(),
                category,
            )
            .with_image_url(image_url);

            results.push(new_result);
        }

        Ok(results)
    }

    fn get_pagination_token(&self, response: &str) -> Result<Option<String>, RetailerError> {
        let response_objects = serde_json::from_str::<ApiResponse>(response)?;
        let pagination_info = response_objects.data.site.products.page_info;

        match pagination_info.has_next_page {
            true => Ok(pagination_info.end_cursor),
            false => Ok(None),
        }
    }
}
