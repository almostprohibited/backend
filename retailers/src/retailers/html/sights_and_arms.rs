// This is woocommerce but they decided to just not have category pages
// and instead list everything under a single page

use async_trait::async_trait;
use common::result::{
    base::{CrawlResult, Price},
    enums::{Category, RetailerName},
};
use crawler::request::{Request, RequestBuilder};
use itertools::any;
use scraper::{ElementRef, Html, Selector};
use tracing::debug;

use crate::{
    errors::RetailerError,
    structures::{HtmlRetailer, HtmlRetailerSuper, HtmlSearchQuery, Retailer},
    utils::{
        conversions::price_to_cents,
        ecommerce::WooCommerce,
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

const URL: &str = "https://sightsandarms.com/shop/page/{page}/";

pub struct SightsAndArms;

impl Default for SightsAndArms {
    fn default() -> Self {
        Self::new()
    }
}

impl SightsAndArms {
    pub fn new() -> Self {
        Self {}
    }

    // unicode space in str replace, not regular space
    // if your site uses &nbsp; in 2026, you deserve
    // a red box of 7.62 that contains random corrosive primer
    fn get_price(element: ElementRef) -> Result<Price, RetailerError> {
        let price_element = extract_element_from_element(element, "span.price")?;

        let mut price = Price {
            regular_price: 0,
            sale_price: None,
        };

        let regular_non_sale_price =
            extract_element_from_element(price_element, ":scope > span.amount > bdi");

        match regular_non_sale_price {
            Ok(regular_price_element) => {
                price.regular_price =
                    price_to_cents(element_to_text(regular_price_element).replace(" CAD", ""))?;
            }
            Err(_) => {
                let sale_price = extract_element_from_element(
                    price_element,
                    ":scope > ins > span.amount > bdi",
                )?;
                let previous_price = extract_element_from_element(
                    price_element,
                    ":scope > del > span.amount > bdi",
                )?;

                price.regular_price =
                    price_to_cents(element_to_text(previous_price).replace(" CAD", ""))?;
                price.sale_price = Some(price_to_cents(
                    element_to_text(sale_price).replace(" CAD", ""),
                )?);
            }
        }

        Ok(price)
    }
}

impl HtmlRetailerSuper for SightsAndArms {}

impl Retailer for SightsAndArms {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::SightsAndArms
    }
}

#[async_trait]
impl HtmlRetailer for SightsAndArms {
    async fn build_page_request(
        &self,
        page_num: u64,
        _search_term: &HtmlSearchQuery,
    ) -> Result<Request, RetailerError> {
        let url = URL.replace("{page}", (page_num + 1).to_string().as_str());

        debug!("Setting page to {}", url);

        let request = RequestBuilder::new().set_url(url).build();

        Ok(request)
    }

    async fn parse_response(
        &self,
        response: &String,
        _search_term: &HtmlSearchQuery,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut results: Vec<CrawlResult> = Vec::new();

        let fragment = Html::parse_document(response);

        let product_selector = Selector::parse("ul.products > li.product.instock").unwrap();

        for element in fragment.select(&product_selector) {
            let title_element = extract_element_from_element(element, "a.ast-loop-product__link")?;
            let link = element_extract_attr(title_element, "href")?;
            let name = element_to_text(title_element);

            if extract_element_from_element(element, "span.price").is_err() {
                debug!("Skipping {name} as it is a listed product with no price");

                continue;
            }

            let price = Self::get_price(element)?;

            if price.regular_price == 0 {
                debug!("Skipping {name} as it has price set to $0");

                continue;
            }

            let category_element =
                extract_element_from_element(element, "span.ast-woo-product-category")?;
            let category_text = element_to_text(category_element).to_lowercase();

            let mut category = Category::Other;

            // what a mess, they don't even categorize some of their rifles
            // so this if block is a best attempt
            if any(
                ["firearm", "used & consigned", "non-restricted", "shotguns"],
                |prefix| category_text.starts_with(prefix),
            ) && !category_text.contains("courses")
            {
                category = Category::Firearm;
            }

            let image_element =
                extract_element_from_element(element, "img.attachment-woocommerce_thumbnail")?;
            let image_link = element_extract_attr(image_element, "data-src")?;

            let result = CrawlResult::new(name, link, price, self.get_retailer_name(), category)
                .with_image_url(image_link);

            results.push(result);
        }

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        vec![HtmlSearchQuery {
            term: "".into(),
            category: Category::Other,
        }]
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        WooCommerce::parse_max_pages(response)
    }
}
