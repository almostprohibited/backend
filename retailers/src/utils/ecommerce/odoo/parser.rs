use std::{sync::LazyLock, time::Duration};

use common::result::{
    base::{CrawlResult, Price},
    enums::RetailerName,
};
use crawler::{request::RequestBuilder, traits::HttpMethod, unprotected::UnprotectedCrawler};
use itertools::Itertools;
use regex::Regex;
use scraper::{Html, Selector};
use tokio::time::sleep;

use crate::{
    errors::RetailerError,
    structures::HtmlSearchQuery,
    utils::{
        conversions::{price_to_cents, string_to_u64},
        ecommerce::odoo::api_structs::{
            QuickViewResponse, VariantResponse, get_quick_view_api_body, get_variant_api_body,
        },
        helpers::clean_url,
        html::{element_extract_attr, extract_element_from_element},
        regex::unwrap_regex_capture,
    },
};

struct ProductBaseInfo {
    product_id: u64,
    template_id: u64,
    url: String,
    image_url: String,
}

static NAME_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:\[[^]]+]\s*)?(.+)$").expect("Static regex to compile"));

pub(crate) struct Odoo {
    base_url: String,
    quick_view_url: String,
    variant_url: String,
    retailer: RetailerName,
}

impl Odoo {
    pub(crate) fn new(
        base_url: &str,
        quick_view_url: &str,
        variant_url: &str,
        retailer: RetailerName,
    ) -> Self {
        let mut base = base_url.to_string();

        if base.ends_with("/") {
            base.pop();
        }

        Self {
            base_url: base,
            quick_view_url: quick_view_url.to_string(),
            variant_url: variant_url.to_string(),
            retailer,
        }
    }

    pub(crate) async fn parse_page(
        &self,
        response: &str,
        search_term: &HtmlSearchQuery,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut results: Vec<CrawlResult> = vec![];

        let product_base = self.extract_product_ids(response)?;

        for product in product_base {
            let quick_view_html = self.query_quick_view(product.template_id).await?;

            let quick_view_response = serde_json::from_str::<QuickViewResponse>(&quick_view_html)?;

            let variant_groups = Self::parse_quick_view(&quick_view_response)?;

            let result = self
                .process_variants(variant_groups, product, search_term)
                .await?;

            results.extend(result);

            sleep(Duration::from_secs(1)).await;
        }

        Ok(results)
    }

    fn extract_product_ids(&self, html: &str) -> Result<Vec<ProductBaseInfo>, RetailerError> {
        let mut products: Vec<ProductBaseInfo> = vec![];

        let html = Html::parse_document(html);

        let product_selector = Selector::parse("div.tp-product-top").unwrap();

        for element in html.select(&product_selector) {
            let wishlist_element = extract_element_from_element(element, "button.o_add_wishlist")?;
            let product_template_id = string_to_u64(element_extract_attr(
                wishlist_element,
                "data-product-template-id",
            )?)?;
            let product_id = string_to_u64(element_extract_attr(
                wishlist_element,
                "data-product-product-id",
            )?)?;

            let product_url_element =
                extract_element_from_element(element, "a.tp-product-image-container")?;

            let raw_product_url = format!(
                "{}{}",
                self.base_url,
                element_extract_attr(product_url_element, "href")?
            );

            let product_url = clean_url(&raw_product_url);

            let image_element =
                extract_element_from_element(product_url_element, "img.tp-product-image")?;
            let image_url = format!(
                "{}{}",
                self.base_url,
                element_extract_attr(image_element, "src")?
            );

            products.push(ProductBaseInfo {
                product_id,
                template_id: product_template_id,
                url: product_url,
                image_url: image_url,
            });
        }

        Ok(products)
    }

    fn parse_quick_view(
        quick_view_response: &QuickViewResponse,
    ) -> Result<Vec<Vec<u64>>, RetailerError> {
        let html = Html::parse_document(&quick_view_response.result);
        let variant_selectors =
            Selector::parse("ul[data-attribute_exclusions] li[data-attribute_id]").unwrap();

        let mut variant_groups: Vec<Vec<u64>> = vec![];

        for element in html.select(&variant_selectors) {
            let variant_data_selector = Selector::parse("*[data-value_id]").unwrap();

            let mut inner_variant_group: Vec<u64> = vec![];

            for variant_element in element.select(&variant_data_selector) {
                let variant_id =
                    string_to_u64(element_extract_attr(variant_element, "data-value_id")?)?;

                inner_variant_group.push(variant_id);
            }

            variant_groups.push(inner_variant_group);
        }

        Ok(variant_groups)
    }

    async fn process_variants(
        &self,
        variant_groups: Vec<Vec<u64>>,
        product: ProductBaseInfo,
        search_term: &HtmlSearchQuery,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut results: Vec<CrawlResult> = vec![];

        for variants in variant_groups
            .into_iter()
            .multi_cartesian_product()
            .collect::<Vec<_>>()
        {
            let variant_api_response = self
                .query_variant(variants, product.product_id, product.template_id)
                .await?;

            let deserialized_variant =
                serde_json::from_str::<VariantResponse>(&variant_api_response)?;

            if deserialized_variant.result.is_combination_possible
                && deserialized_variant.result.free_qty > 0.0
            {
                let name =
                    unwrap_regex_capture(&NAME_REGEX, &deserialized_variant.result.display_name)?;

                let result = CrawlResult::new(
                    name,
                    product.url.clone(),
                    Self::parse_price(deserialized_variant)?,
                    self.retailer,
                    search_term.category,
                )
                .with_image_url(product.image_url.clone());

                results.push(result);
            }

            sleep(Duration::from_secs(1)).await;
        }

        Ok(results)
    }

    fn parse_price(variant_obj: VariantResponse) -> Result<Price, RetailerError> {
        let mut price = Price {
            regular_price: price_to_cents(variant_obj.result.list_price.to_string())?,
            sale_price: None,
        };

        if variant_obj.result.compare_list_price > 0.0 {
            price.sale_price = Some(price.regular_price);
            price.regular_price =
                price_to_cents(variant_obj.result.compare_list_price.to_string())?;
        }

        Ok(price)
    }

    async fn query_quick_view(&self, template_id: u64) -> Result<String, RetailerError> {
        let request = RequestBuilder::new()
            .set_url(self.quick_view_url.clone())
            .set_method(HttpMethod::POST)
            .set_json_body(get_quick_view_api_body(template_id))
            .build();

        let crawler = UnprotectedCrawler::make_web_request(request).await?;

        Ok(crawler.body)
    }

    async fn query_variant(
        &self,
        variant_array: Vec<u64>,
        product_id: u64,
        template_id: u64,
    ) -> Result<String, RetailerError> {
        let request = RequestBuilder::new()
            .set_url(self.variant_url.clone())
            .set_method(HttpMethod::POST)
            .set_json_body(get_variant_api_body(variant_array, product_id, template_id))
            .build();

        let crawler = UnprotectedCrawler::make_web_request(request).await?;

        Ok(crawler.body)
    }
}
