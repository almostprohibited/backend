use async_trait::async_trait;
use common::result::{
    base::CrawlResult,
    enums::{Category, RetailerName},
};
use crawler::{
    request::{Request, RequestBuilder},
    unprotected::UnprotectedCrawler,
};
use scraper::{Html, Selector};
use tracing::{debug, warn};

use crate::{
    errors::RetailerError,
    structures::{HtmlRetailer, HtmlRetailerSuper, HtmlSearchQuery, Retailer},
    utils::{
        ecommerce::{WooCommerce, WooCommerceBuilder, WooCommerceNested},
        generic_sitemap::get_search_queries,
        html::{element_extract_attr, element_to_text, extract_element_from_element},
        sucuri_cookie::get_sucuri_cookie,
    },
};

const SITE_MAP: &str = "https://www.gotenda.com/product_cat-sitemap.xml";
const PRODUCT_BASE_URL: &str = "https://www.gotenda.com/product-category/";
const BASE_URL: &str = "https://www.gotenda.com/";
const URL: &str = "https://www.gotenda.com/product-category/{category}/page/{page}/?stock=instock";

pub struct Tenda {
    search_terms: Vec<HtmlSearchQuery>,
}

impl Default for Tenda {
    fn default() -> Self {
        Self::new()
    }
}

impl Tenda {
    pub fn new() -> Self {
        Self {
            search_terms: Vec::new(),
        }
    }

    async fn get_search_queries() -> Result<Vec<HtmlSearchQuery>, RetailerError> {
        get_search_queries(SITE_MAP, PRODUCT_BASE_URL, |link| {
            if link.contains("/watches") || link.contains("/casual") || link.contains("/hats") {
                return None;
            }

            if link.starts_with("accessories/")
                || link.starts_with("reloading/")
                || link.starts_with("optic/")
            {
                return Some(HtmlSearchQuery {
                    term: link,
                    category: Category::Other,
                });
            } else if (link.starts_with("ammunition") && link.contains("-ammo"))
                || link.contains("/snap-cap")
            {
                return Some(HtmlSearchQuery {
                    term: link,
                    category: Category::Ammunition,
                });
            } else if link.starts_with("firearms-") {
                return Some(HtmlSearchQuery {
                    term: link,
                    category: Category::Firearm,
                });
            };

            None
        })
        .await
    }
}

impl HtmlRetailerSuper for Tenda {}

#[async_trait]
impl Retailer for Tenda {
    async fn init(&mut self) -> Result<(), RetailerError> {
        let cookie = get_sucuri_cookie(BASE_URL).await?;

        UnprotectedCrawler::set_cookie(BASE_URL, &cookie);

        debug!("Using cookie: {cookie}");

        self.search_terms.extend(Self::get_search_queries().await?);

        debug!("{:?}", self.search_terms);

        Ok(())
    }

    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::Tenda
    }
}

#[async_trait]
impl HtmlRetailer for Tenda {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &HtmlSearchQuery,
    ) -> Result<Request, RetailerError> {
        let url = URL
            .replace("{category}", &search_term.term)
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

        let fragment = Html::parse_document(response);

        let mut product_variants: Vec<String> = vec![];

        let product_selector = Selector::parse("ul.products > li.product.instock").unwrap();

        let woocommerce_helper = WooCommerceBuilder::default()
            .with_product_url_selector("h3.products-title > a")
            .with_product_name_selector("h3.products-title > a")
            .with_image_url_selector("figure.products-img > a > img")
            .build();

        for element in fragment.select(&product_selector) {
            let name_element = extract_element_from_element(element, "h3.products-title > a")?;
            let name = element_to_text(name_element).to_lowercase();

            if name.contains("sticker") && name.contains("draw") {
                debug!("Skipping {name} as it contains sticker draw");
                continue;
            }

            let add_cart_button = extract_element_from_element(element, "a.add_to_cart_button")?;

            if element_to_text(add_cart_button).to_lowercase() == "select options" {
                if let Ok(product_link) = element_extract_attr(add_cart_button, "href") {
                    product_variants.push(product_link);

                    continue;
                } else {
                    warn!(
                        "Failed to extract link for nested product, falling back to normal product"
                    );
                }
            }

            results.push(woocommerce_helper.parse_product(
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
        // sucks to clone this, but I don't remember if this is run in a loop
        self.search_terms.clone()
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        WooCommerce::parse_max_pages(response)
    }
}
