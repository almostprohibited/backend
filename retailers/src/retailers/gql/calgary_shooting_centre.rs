use async_trait::async_trait;
use common::result::{
    base::CrawlResult,
    enums::{Category, RetailerName},
};
use crawler::request::Request;

use crate::{
    errors::RetailerError,
    structures::{GqlRetailer, GqlRetailerSuper, Retailer},
    utils::ecommerce::bigcommerce::{
        gql_helpers::{build_request, get_gql_token},
        gql_structs::{ApiResponse, CategoryMatch},
    },
};

const MAIN_URL: &str = "https://store.theshootingcentre.com/";

fn product_classifier(path_node: &str) -> CategoryMatch {
    if path_node.starts_with("/clearance/")
        || path_node.starts_with("/gear/camp-hike/")
        || path_node.starts_with("/gear/apparel/")
    {
        return CategoryMatch::Ignore;
    }

    if path_node.starts_with("/firearms/") {
        return CategoryMatch::Match(Category::Firearm);
    }

    if path_node.starts_with("/ammunition/") {
        return CategoryMatch::Match(Category::Ammunition);
    }

    CategoryMatch::Match(Category::Other)
}

pub struct CalgaryShootingCentre {
    auth_token: String,
}

impl Default for CalgaryShootingCentre {
    fn default() -> Self {
        Self::new()
    }
}

impl CalgaryShootingCentre {
    pub fn new() -> Self {
        Self {
            auth_token: String::new(),
        }
    }
}

impl GqlRetailerSuper for CalgaryShootingCentre {}

#[async_trait]
impl Retailer for CalgaryShootingCentre {
    async fn init(&mut self) -> Result<(), RetailerError> {
        self.auth_token = get_gql_token(MAIN_URL).await?;

        Ok(())
    }

    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::CalgaryShootingCentre
    }
}

#[async_trait]
impl GqlRetailer for CalgaryShootingCentre {
    async fn build_page_request(
        &self,
        pagination_token: Option<String>,
    ) -> Result<Request, RetailerError> {
        let mut gql_url = MAIN_URL.to_string();

        if !gql_url.ends_with("/") {
            gql_url.push('/');
        }

        build_request(
            &format!("{gql_url}graphql"),
            &self.auth_token,
            pagination_token,
        )
    }

    async fn parse_response(&self, response: &str) -> Result<Vec<CrawlResult>, RetailerError> {
        let response_objects = serde_json::from_str::<ApiResponse>(response)?;

        let mut products = response_objects.get_products(
            MAIN_URL,
            self.get_retailer_name(),
            product_classifier,
        )?;

        products.iter_mut().for_each(|product| {
            if product.category == Category::Ammunition {
                product.update_name(&format!("{}rds", product.name));
            }
        });

        Ok(products)
    }

    fn get_pagination_token(&self, response: &str) -> Result<Option<String>, RetailerError> {
        let response_objects = serde_json::from_str::<ApiResponse>(response)?;

        Ok(response_objects.get_pagination_token())
    }
}
