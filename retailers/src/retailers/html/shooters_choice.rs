use std::sync::LazyLock;

use async_trait::async_trait;
use common::result::{
    base::CrawlResult,
    enums::{Category, RetailerName},
};
use crawler::request::{Request, RequestBuilder};
use regex::Regex;
use scraper::{Html, Selector};
use tracing::{debug, warn};

use crate::{
    errors::RetailerError,
    structures::{HtmlRetailer, HtmlRetailerSuper, HtmlSearchQuery, Retailer},
    utils::{
        conversions::string_to_u64,
        ecommerce::{WooCommerce, WooCommerceBuilder, WooCommerceNested},
        html::{element_extract_attr, extract_element_from_element},
        regex::unwrap_regex_capture,
    },
};

static PAGE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"/page/(\d+)/").expect("Regex to compile"));

const URL: &str = "https://shooterschoice.com/category/{category}/page/{page}/?stock=instock";

pub struct ShootersChoice;

impl Default for ShootersChoice {
    fn default() -> Self {
        Self::new()
    }
}

impl ShootersChoice {
    pub fn new() -> Self {
        Self {}
    }
}

impl HtmlRetailerSuper for ShootersChoice {}

impl Retailer for ShootersChoice {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::ShootersChoice
    }
}

#[async_trait]
impl HtmlRetailer for ShootersChoice {
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

        let product_selector = Selector::parse("ul.products > li.product.instock").unwrap();

        let mut product_variants: Vec<String> = vec![];

        let woocommerce = WooCommerceBuilder::default()
            .with_image_url_selector("div.post_thumb > a > img")
            .with_product_name_selector("h2.woocommerce-loop-product__title > a")
            .with_product_url_selector("h2.woocommerce-loop-product__title > a")
            .build();

        for element in fragment.select(&product_selector) {
            if let Ok(add_cart_button) =
                extract_element_from_element(element, "a.add_to_cart_button.product_type_variable")
            {
                if let Ok(product_link) = element_extract_attr(add_cart_button, "href") {
                    product_variants.push(product_link);

                    continue;
                } else {
                    warn!(
                        "Failed to extract link for nested product, falling back to normal product"
                    );
                }
            }

            results.push(woocommerce.parse_product(
                element,
                self.get_retailer_name(),
                search_term.category,
            )?);
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
                term: "4022-firearms".into(),
                category: Category::Firearm,
            },
            HtmlSearchQuery {
                term: "4021-ammunition".into(),
                category: Category::Ammunition,
            },
        ];

        // I'm sorry people, there are going to be random dog leashes here
        // since they sell these for some reason, and I don't want to deal
        // with filtering them out
        for other in ["4027-accessories", "4030-optics", "reloading-accessories"] {
            search.push(HtmlSearchQuery {
                term: other.into(),
                category: Category::Other,
            });
        }

        search
    }

    // their website is so weird, they have two different
    // pagination menus depending on what category you are looking at
    // we have to check both, because there are pages that have a custom one
    // pages that have the standard one, and pages that have both???
    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        let non_standard_pages = get_non_standard_pages(response);

        if let Err(err) = non_standard_pages {
            warn!("Failed to find page numbers ({err}), falling back to normal parsing");

            return WooCommerce::parse_max_pages(response);
        }

        non_standard_pages
    }
}

fn get_non_standard_pages(response: &str) -> Result<u64, RetailerError> {
    let html = Html::parse_document(response);

    let next_page_element =
        extract_element_from_element(html.root_element(), "nav#pagination > a.pager_next")?;

    let link = element_extract_attr(next_page_element, "href")?;

    let page_number = string_to_u64(unwrap_regex_capture(&PAGE_REGEX, &link)?)?;

    Ok(page_number)
}
