use crate::{
    errors::RetailerError,
    structures::{HtmlRetailer, HtmlRetailerSuper, HtmlSearchQuery, Retailer},
    utils::{
        conversions::price_to_cents,
        ecommerce::{
            bigcommerce::BigCommerce,
            bigcommerce_nested::{BigCommerceNested, NestedProduct},
        },
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};
use async_trait::async_trait;
use common::result::{
    base::{CrawlResult, Price},
    enums::{Category, RetailerName},
};
use crawler::request::{Request, RequestBuilder};
use scraper::{ElementRef, Html, Selector};

const PAGE_LIMIT: u64 = 100;
const MAIN_URL: &str =
    "https://store.theshootingcentre.com/{category}/?limit={page_limit}&mode=6&page={page}";
const API_URL: &str =
    "https://store.theshootingcentre.com/remote/v1/product-attributes/{product_id}";
const CART_URL: &str = "https://store.theshootingcentre.com/cart.php";

pub struct CalgaryShootingCentre;

impl CalgaryShootingCentre {
    pub fn new() -> Self {
        Self {}
    }

    /// For regular parcing using HTML elements
    fn get_price_from_element(product_element: ElementRef) -> Result<Price, RetailerError> {
        /*
        <span data-product-price-without-tax="" class="price price--withoutTax price--main">$2,160.00</span>
        <span data-product-non-sale-price-without-tax="" class="price price--non-sale">$2,400.00</span>

        <span data-product-non-sale-price-without-tax="" class="price price--non-sale"></span>
        </span> */

        let price_main = extract_element_from_element(product_element, "span.price--main")?;
        let price_non_sale = extract_element_from_element(product_element, "span.price--non-sale")?;

        let price_str = element_to_text(price_main);
        let price_non_sale_str = element_to_text(price_non_sale);

        let mut price = Price {
            regular_price: price_to_cents(price_str)?,
            sale_price: None,
        };

        if !price_non_sale_str.is_empty() {
            price.sale_price = Some(price.regular_price);
            price.regular_price = price_to_cents(price_non_sale_str)?;
        }

        Ok(price)
    }
}

impl HtmlRetailerSuper for CalgaryShootingCentre {}

impl Retailer for CalgaryShootingCentre {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::CalgaryShootingCentre
    }
}

#[async_trait]
impl HtmlRetailer for CalgaryShootingCentre {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &HtmlSearchQuery,
    ) -> Result<Request, RetailerError> {
        let request: Request = RequestBuilder::new()
            .set_url(
                MAIN_URL
                    .replace("{category}", &search_term.term)
                    .replace("{page_limit}", PAGE_LIMIT.to_string().as_str())
                    .replace("{page}", (page_num + 1).to_string().as_str()),
            )
            .build();

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
            let product_selector = Selector::parse("li.product > article.card").unwrap();

            html.select(&product_selector)
                .map(|inner| inner.html().clone())
                .collect::<Vec<String>>()
        };

        for inner_html in products {
            let html = Html::parse_fragment(&inner_html);
            let product = html.root_element();

            let name_link_element = extract_element_from_element(product, "h4.card-title > a")?;

            let image_element =
                extract_element_from_element(product, "div.card-img-container > img.card-image")?;

            let url = element_extract_attr(name_link_element, "href")?;
            let name = element_to_text(name_link_element);
            let image = element_extract_attr(image_element, "src")?;

            let price_element = extract_element_from_element(product, "span.price--main")?;

            // CSC doesn't list round count in the title: force the crawler to visit the page
            // a `-` indicates variants, meaning we have to visit page
            if element_to_text(price_element).contains("-")
                || search_term.category == Category::Ammunition
            {
                nested_handler.enqueue_product(NestedProduct {
                    name: BigCommerce::get_item_name(product)?,
                    fallback_image_url: BigCommerce::get_image_url(product)?,
                    category: search_term.category,
                    product_url: url,
                });

                continue;
            }

            let price = Self::get_price_from_element(product)?;

            let new_result = CrawlResult::new(
                name,
                url,
                price,
                self.get_retailer_name(),
                search_term.category,
            )
            .with_image_url(image.to_string());

            results.push(new_result);
        }

        results.extend(nested_handler.parse_nested().await?);

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        let mut terms = Vec::from_iter([
            HtmlSearchQuery {
                term: "firearms".into(),
                category: Category::Firearm,
            },
            HtmlSearchQuery {
                term: "ammunition".into(),
                category: Category::Ammunition,
            },
        ]);

        let other_terms = [
            "optics",
            "reloading",
            "gun-parts-accessories",
            "optics-accessories",
        ];

        for other in other_terms {
            terms.push(HtmlSearchQuery {
                term: other.into(),
                category: Category::Other,
            });
        }

        terms
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        BigCommerce::parse_max_pages(response)
    }
}
