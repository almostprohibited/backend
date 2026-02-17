use std::sync::LazyLock;

use crawler::{
    request::{Request, RequestBuilder},
    traits::HttpMethod,
    unprotected::UnprotectedCrawler,
};
use regex::Regex;
use serde_json::json;

use crate::{errors::RetailerError, utils::regex::unwrap_regex_capture};

static PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(
            r"'Authorization'\s*:\s*'Bearer\s+([A-Za-z0-9-_]+\.[A-Za-z0-9-_]+\.[A-Za-z0-9-_]+)'",
        )
        .expect("Regex to not fail creation"),
        Regex::new(r#"graphQLToken\\":\\"(.+?)\\""#).expect("Regex to not fail creation"),
        Regex::new(r#"bearerToken\\":\\"(.+?)\\""#).expect("Regex to not fail creation"),
    ]
});

const PAGINATION_REPLACEMENT_KEY: &str = "{{pagination_token}}";
const API_QUERY_REQUEST: &str = r#"
{
	site {
		products(
			hideOutOfStock: true
            {{pagination_token}}
			first: 50
    ) {
		pageInfo {
			endCursor
			hasNextPage
		}
		edges {
			node {
				categories {
					edges {
						node {
							breadcrumbs(depth: 99) {
								edges {
									node {
										entityId
										name
										path
									}
								}
							}
						}
					}
				}
				name
				inventory {
					isInStock
					hasVariantInventory
				}
				variants {
					edges {
						node {
							options {
								edges {
									node {
										values {
											edges {
												node {
													label
												}
											}
										}
									}
								}
							}
							defaultImage {
								url(width: 800)
							}
							inventory {
								isInStock
							}
							prices(currencyCode: CAD) {
								salePrice {
									value
								}
								basePrice {
									value
								}
							}
						}
					}
				}
				path
				defaultImage {
					url(width: 800)
				}
				prices(currencyCode: CAD) {
					salePrice {
						value
					}
					basePrice {
						value
					}
				}
			}
		}
	}}
}
"#;

pub(crate) fn build_request(
    url: &str,
    token: &str,
    pagination_token: Option<String>,
) -> Result<Request, RetailerError> {
    let mut pagination_entry = String::new();

    if let Some(token) = pagination_token {
        pagination_entry = format!("after: \"{token}\"");
    };

    let request_json = json!({
        "query": API_QUERY_REQUEST.replace(PAGINATION_REPLACEMENT_KEY, &pagination_entry)
    });

    let authorization_header = format!("Bearer {token}");

    let request = RequestBuilder::new()
        .set_url(url)
        .set_method(HttpMethod::POST)
        .set_headers(
            [
                ("Content-Type".into(), "application/json".into()),
                ("Authorization".into(), authorization_header),
            ]
            .as_ref(),
        )
        .set_json_body(request_json)
        .build();

    Ok(request)
}

pub(crate) async fn get_gql_token(url: &str) -> Result<String, RetailerError> {
    let request = RequestBuilder::new().set_url(url).build();
    let response = UnprotectedCrawler::make_web_request(request).await?.body;

    for regex_pattern in PATTERNS.iter() {
        if let Ok(token) = unwrap_regex_capture(&regex_pattern, &response) {
            return Ok(token);
        }
    }

    Err(RetailerError::GeneralError(
        "Body missing valid GQL token".to_string(),
    ))
}
