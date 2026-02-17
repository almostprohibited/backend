use common::result::{
    base::{CrawlResult, Price},
    enums::{Category, RetailerName},
};
use itertools::Itertools;
use serde::Deserialize;
use tracing::debug;

use crate::{errors::RetailerError, utils::conversions::price_to_cents};

// pretty sure this is Prophet River's default no image URL
// we can just reuse it for all
const DEFAULT_IMAGE_URL: &str = "https://cdn11.bigcommerce.com/s-dcynby20nc/stencil/be1fd970-0d6b-013e-f9b9-6613132a0701/e/092afc30-45f5-013e-ca76-52b5c4b168da/img/ProductDefault.gif";

#[derive(Deserialize, Debug)]
pub(crate) struct ApiResponse {
    data: ApiData,
}

impl ApiResponse {
    pub(crate) fn get_products(
        &self,
        main_url: &str,
        retailer: RetailerName,
        product_classifier: fn(&str) -> Option<Category>,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut main_url = main_url.to_string();

        if main_url.ends_with("/") {
            main_url.pop();
        }

        let products: Vec<&ApiProductNode> = self
            .data
            .site
            .products
            .edges
            .iter()
            .map(|edge| &edge.node)
            .collect();

        let mut results: Vec<CrawlResult> = vec![];

        for product in products {
            if !product.is_in_stock() {
                continue;
            }

            let Some(category) = product.get_category(product_classifier) else {
                continue;
            };

            debug!("{:?}", product);

            let url = format!("{main_url}{}", product.path);

            let main_image_url = match &product.default_image {
                Some(api_image) => api_image.url.clone(),
                None => DEFAULT_IMAGE_URL.into(),
            };

            if product.has_variant() {
                for variant in &product.variants.edges {
                    let variant_node = &variant.node;
                    if !variant_node.inventory.is_in_stock {
                        continue;
                    }

                    let variant_image = match &variant_node.default_image {
                        Some(image) => image.url.clone(),
                        None => main_image_url.clone(),
                    };

                    let new_result = CrawlResult::new(
                        format!(
                            "{} - {}",
                            product.name.clone(),
                            variant_node.get_variant_name()
                        ),
                        url.clone(),
                        variant_node.prices.get_price()?,
                        retailer,
                        category,
                    )
                    .with_image_url(variant_image);

                    results.push(new_result);
                }
            } else {
                let new_result = CrawlResult::new(
                    product.name.clone(),
                    url,
                    product.get_price()?,
                    retailer,
                    category,
                )
                .with_image_url(main_image_url);

                results.push(new_result);
            }
        }

        Ok(results)
    }

    pub(crate) fn get_pagination_token(&self) -> Option<String> {
        self.data.site.products.page_info.end_cursor.clone()
    }
}

#[derive(Deserialize, Debug)]
struct ApiData {
    site: ApiSite,
}

#[derive(Deserialize, Debug)]
struct ApiSite {
    products: ApiProducts,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ApiProducts {
    page_info: ApiPageInfo,
    edges: Vec<ApiProductsEdge>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ApiPageInfo {
    end_cursor: Option<String>,
}

#[derive(Deserialize, Debug)]
struct ApiProductsEdge {
    node: ApiProductNode,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ApiProductNode {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) default_image: Option<ApiImage>,
    categories: ApiCategories,
    inventory: ApiInventory,
    prices: ApiProductPrice,

    variants: ApiVariants,
}

#[derive(Deserialize, Debug)]
struct ApiVariants {
    edges: Vec<ApiVariantEdge>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ApiVariantEdge {
    node: ApiVariant,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ApiVariant {
    default_image: Option<ApiImage>,
    inventory: ApiVariantInventory,
    prices: ApiProductPrice,

    options: ApiOptions,
}

impl ApiVariant {
    fn get_variant_name(&self) -> String {
        let mut parts: Vec<String> = vec![];

        for edge in &self.options.edges {
            // hope that this only contains a single element
            for node in &edge.node.values.edges {
                parts.push(node.node.label.clone());
            }
        }

        parts.join(" - ")
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ApiVariantInventory {
    is_in_stock: bool,
}

#[derive(Deserialize, Debug)]
struct ApiOptions {
    edges: Vec<ApiOptionEdge>,
}

#[derive(Deserialize, Debug)]
struct ApiOptionEdge {
    node: ApiOption,
}

#[derive(Deserialize, Debug)]
struct ApiOption {
    values: ApiOptionValues,
}

#[derive(Deserialize, Debug)]
struct ApiOptionValues {
    edges: Vec<ApiOptionValueEdge>,
}

#[derive(Deserialize, Debug)]
struct ApiOptionValueEdge {
    node: ApiOptionValue,
}

#[derive(Deserialize, Debug)]
struct ApiOptionValue {
    label: String,
}

impl ApiProductNode {
    fn is_in_stock(&self) -> bool {
        self.inventory.is_in_stock
    }

    fn has_variant(&self) -> bool {
        self.inventory.has_variant_inventory
    }

    fn get_price(&self) -> Result<Price, RetailerError> {
        self.prices.get_price()
    }

    fn get_category<F>(&self, product_classifier: F) -> Option<Category>
    where
        F: Fn(&str) -> Option<Category>,
    {
        // so there should only be a single edge, but I can't
        // know for sure because the GQL response says that
        // it's a list, so I just have to belive the API
        for edge in &self.categories.edges {
            let mut breadcrumbs = edge.node.breadcrumbs.edges.iter().cloned().collect_vec();
            breadcrumbs.reverse();

            for path_obj in breadcrumbs {
                let path_node = &path_obj.node;

                let result = product_classifier(
                    path_node
                        .path
                        .clone()
                        .unwrap_or_default()
                        .to_lowercase()
                        .as_str(),
                );

                if result.is_some() {
                    return result;
                }
            }
        }

        None
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ApiProductPrice {
    sale_price: Option<ApiPrice>,
    base_price: ApiPrice,
}

impl ApiProductPrice {
    fn get_price(&self) -> Result<Price, RetailerError> {
        let mut price = Price {
            regular_price: price_to_cents(self.base_price.value.to_string())?,
            sale_price: None,
        };

        if let Some(sale_price) = &self.sale_price {
            price.sale_price = Some(price_to_cents(sale_price.value.to_string())?);
        }

        Ok(price)
    }
}

#[derive(Deserialize, Debug)]
struct ApiPrice {
    value: f32,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ApiInventory {
    is_in_stock: bool,
    has_variant_inventory: bool,
}

#[derive(Deserialize, Debug)]
pub(crate) struct ApiImage {
    pub(crate) url: String,
}

#[derive(Deserialize, Debug)]
struct ApiCategories {
    edges: Vec<ApiCategoriesEdge>,
}

#[derive(Deserialize, Debug)]
struct ApiCategoriesEdge {
    node: ApiCategoriesNode,
}

#[derive(Deserialize, Debug)]
struct ApiCategoriesNode {
    breadcrumbs: ApiCategoriesBreadcrumbs,
}

#[derive(Deserialize, Debug)]
struct ApiCategoriesBreadcrumbs {
    edges: Vec<ApiCategoriesBreadcrumbsEdge>,
}

#[derive(Deserialize, Debug, Clone)]
struct ApiCategoriesBreadcrumbsEdge {
    node: ApiCategoriesBreadcrumbsNode,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct ApiCategoriesBreadcrumbsNode {
    // Un-used until I decide whether or not I want
    // string parsing, or ID parsing to determine
    // category type
    // pub(super) entity_id: u64,
    // pub(super) name: String,
    path: Option<String>,
}
