use async_trait::async_trait;
use common::result::{
    base::{CrawlResult, Price},
    enums::{Category, RetailerName},
};
use crawler::request::{Request, RequestBuilder};
use itertools::Itertools;
use scraper::{ElementRef, Html, Selector};
use tracing::debug;

use crate::{
    errors::RetailerError,
    structures::{HtmlRetailer, HtmlRetailerSuper, HtmlSearchQuery, Retailer},
    utils::{
        conversions::{price_to_cents, string_to_u64},
        helpers::clean_url,
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

// they have a &product_sold=3175 to indicate non-sold items but I
// won't be using it, not because it's on the robots.txt (I don't care)
// but it's a random number in the query
//
// they also have an API that returns SSR HTML, but this is fine
const URL: &str =
    "https://ellwoodepps.com/{category}.html/?limit=100&dir=asc&order=product_sold&p={page}";

const IMAGE_BASE: &str = "https://img.ellwoodepps.com/i/_/_/";

pub struct EllwoodEpps;

impl Default for EllwoodEpps {
    fn default() -> Self {
        Self::new()
    }
}

impl EllwoodEpps {
    pub fn new() -> Self {
        Self {}
    }

    // checks if add cart button actually says add cart
    fn add_cart_button_valid(element: ElementRef) -> bool {
        let Ok(cart_button) =
            extract_element_from_element(element, "td.firearm-addtocart > button")
        else {
            return false;
        };

        element_to_text(cart_button).to_ascii_lowercase() == "add to cart"
    }

    fn get_price(element: ElementRef) -> Result<Price, RetailerError> {
        let price_wrapper = extract_element_from_element(element, "div.price-box")?;

        let price =
            match extract_element_from_element(price_wrapper, "p.special-price > span.price") {
                Ok(sale_price_element) => {
                    let regular_price_element =
                        extract_element_from_element(price_wrapper, "p.old-price span.price")?;

                    let sale_price = element_to_text(sale_price_element);
                    let regular_price = element_to_text(regular_price_element);

                    Price {
                        regular_price: price_to_cents(regular_price)?,
                        sale_price: Some(price_to_cents(sale_price)?),
                    }
                }
                Err(_) => {
                    let price_element = extract_element_from_element(
                        price_wrapper,
                        "span.regular-price > span.price",
                    )?;
                    let regular_price = element_to_text(price_element);

                    Price {
                        regular_price: price_to_cents(regular_price)?,
                        sale_price: None,
                    }
                }
            };

        Ok(price)
    }

    fn get_name(
        product_element: ElementRef,
        name: &str,
        category: Category,
    ) -> Result<String, RetailerError> {
        if !matches!(category, Category::Firearm) {
            return Ok(name.to_string());
        };

        let selector = Selector::parse("td.firearm-item").unwrap();

        let mut attributes = product_element.select(&selector);

        let Some(caliber_element) = attributes.nth(1) else {
            debug!("Missing second firearm attribute element, returning original name {name}");

            return Ok(name.to_string());
        };

        let caliber = element_to_text(caliber_element);

        Ok(format!("{name} - {caliber}"))
    }

    // TODO: this method sucks, consider rewriting
    // it turns:
    //   https://img.ellwoodepps.com/i/a/b/CCC/A/12345/tb/001.png
    // into:
    //   https://img.ellwoodepps.com/i/_/_/CCC/A/12345/lg/001.jpg
    fn get_image(element: ElementRef) -> Result<String, RetailerError> {
        let image_element = extract_element_from_element(element, "a.product-image > img")?;
        let thumbnail_url = element_extract_attr(image_element, "src")?;

        let exploded_uri = thumbnail_url.split("/");

        if let Some(image_id) = exploded_uri.clone().nth(6)
            && image_id == "_"
        {
            return Ok(thumbnail_url);
        }

        let tail = exploded_uri
            .tail(5)
            .join("/")
            .replace(".png", ".jpg")
            .replace("/tb/", "/lg/");

        Ok(format!("{}{}", IMAGE_BASE, tail))
    }
}

impl HtmlRetailerSuper for EllwoodEpps {}

impl Retailer for EllwoodEpps {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::EllwoodEpps
    }
}

#[async_trait]
impl HtmlRetailer for EllwoodEpps {
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

    // this will introduce duplicates into the system
    // it's just how ellwood epps catalogues their items
    // for example, they have 4 alcors that are all the same
    // SKU number, but different product IDs
    async fn parse_response(
        &self,
        response: &String,
        search_term: &HtmlSearchQuery,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut results: Vec<CrawlResult> = Vec::new();

        let fragment = Html::parse_document(response);

        let product_selector = Selector::parse("ol#products-list > li.item").unwrap();

        for element in fragment.select(&product_selector) {
            if !Self::add_cart_button_valid(element) {
                debug!("Product does not contain add cart button");

                continue;
            }

            let name_element = extract_element_from_element(element, "td.firearm-name > a")?;
            let name = element_to_text(name_element);

            let url = clean_url(&element_extract_attr(name_element, "href")?);

            let price = Self::get_price(element)?;

            let result = CrawlResult::new(
                Self::get_name(element, &name, search_term.category)?,
                url,
                price,
                self.get_retailer_name(),
                search_term.category,
            )
            .with_image_url(Self::get_image(element)?);

            results.push(result);
        }

        Ok(results)
    }

    // they have a sitemap, but I don't feel like using it
    // it's easier to do this
    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        let mut terms: Vec<HtmlSearchQuery> = vec![];

        // reloading stuff is also captured in this category
        // since they removed the dedicated reloading subcategory
        // TODO: deal with this later, quick fix
        for ammo in ["hunting/ammunition"] {
            terms.push(HtmlSearchQuery {
                term: ammo.to_string(),
                category: Category::Ammunition,
            });
        }

        for firearm_category in ["hunting/firearms"] {
            terms.push(HtmlSearchQuery {
                term: firearm_category.to_string(),
                category: Category::Firearm,
            });
        }

        for other in ["hunting/accessories"] {
            terms.push(HtmlSearchQuery {
                term: other.to_string(),
                category: Category::Other,
            });
        }

        terms
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        let html = Html::parse_document(response);
        let root = html.root_element();

        if extract_element_from_element(root, "td.firearm-addtocart-sold").is_ok() {
            return Ok(0);
        }

        let selector = Selector::parse("div.pages li > a:not(.previous):not(.next)").unwrap();

        let mut page_links = html.select(&selector);

        let Some(last_page_element) = page_links.next_back() else {
            return Ok(0);
        };

        string_to_u64(element_to_text(last_page_element))
    }
}
