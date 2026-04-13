use async_trait::async_trait;
use common::result::{
    base::CrawlResult,
    enums::{Category, RetailerName},
};
use crawler::request::{Request, RequestBuilder};
use scraper::{Html, Selector};
use tracing::debug;

use crate::{
    errors::RetailerError,
    structures::{HtmlRetailer, HtmlRetailerSuper, HtmlSearchQuery, Retailer},
    utils::{
        conversions::string_to_u64,
        ecommerce::{WooCommerce, WooCommerceBuilder, WooCommerceNested},
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

const URL: &str = "https://doubletapsports.com/product-category/{category}/page/{page}/";

pub struct DoubleTapSports {}

impl Default for DoubleTapSports {
    fn default() -> Self {
        Self::new()
    }
}

impl DoubleTapSports {
    pub fn new() -> Self {
        Self {}
    }
}

impl HtmlRetailerSuper for DoubleTapSports {}

impl Retailer for DoubleTapSports {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::DoubleTapSports
    }
}

#[async_trait]
impl HtmlRetailer for DoubleTapSports {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &HtmlSearchQuery,
    ) -> Result<Request, RetailerError> {
        let url = URL
            .replace("{category}", &search_term.term)
            .replace("{page}", (page_num + 1).to_string().as_str());

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

        let fragment = Html::parse_document(response);

        // sorry, not dealing with bundles that contain selectable multi-product options
        let simple_product_selector = Selector::parse(
            "ul.products > li:not(.product_tag-bundle).product.instock.product-type-simple",
        )
        .unwrap();

        let woocommerce = WooCommerceBuilder::default()
            .with_image_url_selector("div.image_wrapper > a > div > img")
            .with_product_name_selector("p.title > a")
            .with_product_url_selector("p.title > a")
            .build();

        for element in fragment.select(&simple_product_selector) {
            results.push(woocommerce.parse_product(
                element,
                self.get_retailer_name(),
                search_term.category,
            )?);
        }

        let mut product_variants: Vec<String> = vec![];

        let variant_product_selector = Selector::parse(
            "ul.products > li:not(.product_tag-bundle).product.instock.product-type-variable",
        )
        .unwrap();

        for variable_element in fragment.select(&variant_product_selector) {
            if let Ok(add_cart_button) =
                extract_element_from_element(variable_element, "p.title > a")
            {
                if let Ok(product_link) = element_extract_attr(add_cart_button, "href") {
                    product_variants.push(product_link);

                    continue;
                }
            }
        }

        for link in product_variants {
            results.extend(
                WooCommerce::parse_nested_products(
                    link,
                    search_term.category,
                    self.get_retailer_name(),
                )
                .await?,
            );
        }

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        let mut search: Vec<HtmlSearchQuery> = vec![
            HtmlSearchQuery {
                term: "firearms".into(),
                category: Category::Firearm,
            },
            HtmlSearchQuery {
                term: "ammunition".into(),
                category: Category::Ammunition,
            },
        ];

        for other in [
            "accessories",
            "parts",
            "holsters-pouches-belts",
            "range-gear",
            "reloading",
        ] {
            search.push(HtmlSearchQuery {
                term: other.into(),
                category: Category::Other,
            });
        }

        search
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        let html = Html::parse_fragment(response);

        if extract_element_from_element(html.root_element(), "ul.products > li.product.outofstock")
            .is_ok()
        {
            debug!("Out of stock item in page, ending pagination for category");

            return Ok(0);
        }

        let page_number_selector = Selector::parse("div.pages a.page").unwrap();

        let mut page_links = html.select(&page_number_selector);

        let Some(last_page_element) = page_links.next_back() else {
            return Ok(0);
        };

        string_to_u64(element_to_text(last_page_element))
    }
}
