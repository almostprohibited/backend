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
        ecommerce::{
            bigcommerce::BigCommerce,
            bigcommerce_nested::{BigCommerceNested, NestedProduct},
        },
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

const API_URL: &str = "https://truenortharms.com/remote/v1/product-attributes/{product_id}";
const CART_URL: &str = "https://truenortharms.com/cart.php";
const URL: &str = "https://truenortharms.com/{category}/?page={page}&in_stock=1";

pub struct TrueNorthArms;

impl Default for TrueNorthArms {
    fn default() -> Self {
        Self::new()
    }
}

impl TrueNorthArms {
    pub fn new() -> Self {
        Self {}
    }
}

impl HtmlRetailerSuper for TrueNorthArms {}

impl Retailer for TrueNorthArms {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::TrueNorthArms
    }
}

#[async_trait]
impl HtmlRetailer for TrueNorthArms {
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
        let mut nested_handler =
            BigCommerceNested::new(API_URL, CART_URL, self.get_retailer_name());

        let mut results: Vec<CrawlResult> = Vec::new();

        let products = {
            let html = Html::parse_document(response);
            let product_selector =
                Selector::parse("ul.productGrid > li.product > article.card").unwrap();
            html.select(&product_selector)
                .map(|element| element.html().clone())
                .collect::<Vec<_>>()
        };

        for html_doc in products {
            let product_inner = Html::parse_document(&html_doc);
            let product = product_inner.root_element();

            let title_element = extract_element_from_element(product, "h4.card-title > a")?;

            if element_to_text(title_element).contains("Custom Magpul") {
                continue;
            }

            let cart_button =
                extract_element_from_element(product, "div.card-text.add-to-cart-button")?;
            let button_text = element_to_text(cart_button).to_lowercase();

            let price_element = extract_element_from_element(
                product,
                "div.price-section > span.price.price--withoutTax",
            )?;
            let price_text = element_to_text(price_element);

            if button_text.contains("choose options") || price_text.contains("-") {
                let url = element_extract_attr(title_element, "href")?;

                nested_handler.enqueue_product(NestedProduct {
                    name: BigCommerce::get_item_name(product)?,
                    fallback_image_url: BigCommerce::get_image_url(product)?,
                    category: search_term.category,
                    product_url: url,
                });
            } else if button_text.contains("add to cart") {
                let result = BigCommerce::parse_product(
                    product,
                    self.get_retailer_name(),
                    search_term.category,
                )?;

                results.push(result);
            }
        }

        results.extend(nested_handler.parse_nested().await?);

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        let mut terms = Vec::from_iter([HtmlSearchQuery {
            term: "firearms/non-restricted".into(),
            category: Category::Firearm,
        }]);

        let other = [
            "ar-15-magazines/magazines",
            "magazines/mag-unloaders",
            "ar-15-magazines/mag-accessories",
            "ar-15-magazines/mag-adapters",
            "ar-15-magazines/mag-couplers",
            "ar-15-magazines/magazine-loaders-clips",
            "ar-15-magazines/magazine-parts",
            "glock/magazines/magazine-catch",
            "glock/magazines/mag-springs-insets",
            "glock/magazines/magazine-accessories",
            "glock/magazines/magazines",
            "glock/magazines/magwells",
            "reloading/reloading-components/pistol-rifle-reloading/brass-casings",
            "reloading/reloading-components/pistol-rifle-reloading/bullets-projectiles",
            "reloading/reloading-components/pistol-rifle-reloading/powders",
            "reloading/reloading-components/pistol-rifle-reloading/primers",
            "reloading/tools-equipment",
            "ar-15-ar-308/accessories/targets",
            "ar-15-ar-308/accessories/sling-mounts",
            "ar-15-ar-308/accessories/slings",
            "accessories/scope-accessories/boresights",
            "accessories/scope-accessories/bubble-levels",
            "accessories/scope-accessories/mounts-risers",
            "accessories/scope-accessories/scope-rings",
            "ar-15-ar-308/accessories/safety",
            "ar-15-ar-308/accessories/range-accessories",
            "ar-15-ar-308/accessories/rail-adapters",
            "ar-15-ar-308/accessories/rail-covers",
            "ar-15-ar-308/accessories/optics-red-dots",
            "tools-accessories/accessories/knives-bayonets/",
            "accessories/tools-accessories/gear-armour/",
            "ar-15-ar-308/accessories/flashlights-mounts",
            "ar-15-ar-308/accessories/bipods",
            "tools/wrenches",
            "tools/vice-blocks-fixtures",
            "tools/misc-modifications",
            "tools/machine-tools-bits",
            "tools/hand-tools",
            "tools/hammers-punches",
            "tools/grease-lube-oil",
            "tools/gauges-measurement",
            "tools/boresnakes",
            "tools/armourers-kits",
        ];

        for other_term in other {
            terms.push(HtmlSearchQuery {
                term: other_term.into(),
                category: Category::Other,
            });
        }

        terms
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        BigCommerce::parse_max_pages(response)
    }
}
