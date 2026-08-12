use common::result::{
    base::{CrawlResult, Price},
    enums::{Category, RetailerName},
};
use scraper::{ElementRef, Html, Selector};

use crate::{
    errors::RetailerError,
    utils::{
        conversions::{price_to_cents, string_to_u64},
        html::{
            element_extract_attr, element_to_text, extract_element_from_element,
            match_element_from_list,
        },
    },
};

const VALID_IMAGE_ATTRS: [&str; 5] = [
    "data-src",
    "src",
    "data-wood-src",
    "data-lazy-src",
    "data-src-img",
    // "data-src-webp", // odd that someone not have -img, but will have -webp
];

const DEFAULT_PRODUCT_NAME_SELECTORS: [&str; 3] = [
    "div.product-element-bottom > h3 > a",
    "a.woocommerce-LoopProduct-link > h2.woocommerce-loop-product__title",
    "h2.woocommerce-loop-product__title > a.woocommerce-LoopProduct-link",
];

const DEFAULT_PRODUCT_URL_SELECTORS: [&str; 2] = [
    "div.product-element-bottom > h3 > a",
    "a.woocommerce-LoopProduct-link",
];

const DEFAULT_IMAGE_URL_SELECTORS: [&str; 2] = [
    "a.product-image-link > img",
    "a.woocommerce-LoopProduct-link img",
];

const PRICE_WRAPPER: [&str; 3] = ["span.price", "div.price", "p.price"];

pub(crate) struct WooCommerceBuilder {
    product_name_selector: Vec<String>,
    product_url_selector: Vec<String>,
    image_url_selector: Vec<String>,
}

impl WooCommerceBuilder {
    pub(crate) fn default() -> Self {
        Self {
            product_name_selector: DEFAULT_PRODUCT_NAME_SELECTORS
                .iter()
                .map(|selector| selector.to_string())
                .collect(),
            product_url_selector: DEFAULT_PRODUCT_URL_SELECTORS
                .iter()
                .map(|selector| selector.to_string())
                .collect(),
            image_url_selector: DEFAULT_IMAGE_URL_SELECTORS
                .iter()
                .map(|selector| selector.to_string())
                .collect(),
        }
    }

    pub(crate) fn with_product_name_selector(mut self, selector: impl Into<String>) -> Self {
        self.product_name_selector = vec![selector.into()];

        self
    }

    pub(crate) fn with_product_url_selector(mut self, selector: impl Into<String>) -> Self {
        self.product_url_selector = vec![selector.into()];

        self
    }

    pub(crate) fn with_image_url_selector(mut self, selector: impl Into<String>) -> Self {
        self.image_url_selector = vec![selector.into()];

        self
    }

    pub(crate) fn build(self) -> WooCommerce {
        WooCommerce { options: self }
    }
}

pub(crate) struct WooCommerce {
    options: WooCommerceBuilder,
}

impl WooCommerce {
    fn parse_price(element: ElementRef) -> Result<Price, RetailerError> {
        let mut price = Price {
            regular_price: 0,
            sale_price: None,
        };

        let price_wrapper = match_element_from_list(
            element,
            &PRICE_WRAPPER
                .iter()
                .map(|selector| selector.to_string())
                .collect(),
            RetailerError::HtmlMissingElement("Missing price wrapper".to_string()),
        )?;

        // this is to handle the gun dealer wrapping prices in a <span>
        let price_element =
            match extract_element_from_element(price_wrapper, ":scope > span.electro-price") {
                Ok(element) => element,
                Err(_) => price_wrapper,
            };

        let regular_non_sale_price =
            extract_element_from_element(price_element, ":scope > span.amount");

        match regular_non_sale_price {
            Ok(regular_price_element) => {
                price.regular_price = price_to_cents(element_to_text(regular_price_element))?;
            }
            Err(_) => {
                let sale_price =
                    extract_element_from_element(price_element, ":scope > ins > span.amount")?;
                let previous_price =
                    extract_element_from_element(price_element, ":scope > del > span.amount")?;

                price.regular_price = price_to_cents(element_to_text(previous_price))?;
                price.sale_price = Some(price_to_cents(element_to_text(sale_price))?);
            }
        }

        Ok(price)
    }

    pub(crate) fn parse_max_pages(response: &str) -> Result<u64, RetailerError> {
        let fragment = Html::parse_document(response);
        let page_number_selector =
            Selector::parse("ul.page-numbers > li > a:not(.next):not(.prev).page-numbers").unwrap();

        let mut page_links = fragment.select(&page_number_selector);

        let Some(last_page_element) = page_links.next_back() else {
            return Ok(0);
        };

        string_to_u64(element_to_text(last_page_element))
    }

    fn get_image_url(&self, element: ElementRef) -> Result<String, RetailerError> {
        let image_element = match_element_from_list(
            element,
            &self.options.image_url_selector,
            RetailerError::HtmlMissingElement("Missing valid image element".to_string()),
        )?;

        for attr in VALID_IMAGE_ATTRS {
            if let Ok(data_src) = element_extract_attr(image_element, attr)
                && data_src.starts_with("https")
                && !data_src.contains("lazy")
            {
                return Ok(data_src);
            };
        }

        Err(RetailerError::HtmlElementMissingAttribute(
            "Image element missing valid attribute (check for theme updates)".into(),
            element_to_text(image_element),
        ))
    }

    pub(crate) fn parse_product(
        &self,
        element: ElementRef,
        retailer: RetailerName,
        category: Category,
    ) -> Result<CrawlResult, RetailerError> {
        let url_element = match_element_from_list(
            element,
            &self.options.product_url_selector,
            RetailerError::HtmlMissingElement("Missing valid URL element".to_string()),
        )?;

        let name_element = match_element_from_list(
            element,
            &self.options.product_name_selector,
            RetailerError::HtmlMissingElement("Missing valid name element".to_string()),
        )?;

        let name = element_to_text(name_element);
        let url = element_extract_attr(url_element, "href")?;

        let image_url = self.get_image_url(element)?;

        let new_product =
            CrawlResult::new(name, url, Self::parse_price(element)?, retailer, category)
                .with_image_url(image_url);

        Ok(new_product)
    }
}
