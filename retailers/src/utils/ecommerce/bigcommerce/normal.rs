use common::result::{
    base::{CrawlResult, Price},
    enums::{Category, RetailerName},
};
use scraper::{ElementRef, Html, Selector};
use tracing::debug;

use crate::{
    errors::RetailerError,
    utils::{
        conversions::{price_to_cents, string_to_u64},
        ecommerce::bigcommerce::structs::NestedProduct,
        helpers::clean_url,
        html::{element_extract_attr, element_to_text, match_element_from_list},
    },
};

const VALID_IMAGE_ATTRS: [&str; 5] = [
    "data-src",
    "src",
    "data-wood-src",
    "data-lazy-src",
    "data-src-img",
];

const DEFAULT_PRODUCT_URL_SELECTORS: [&str; 2] =
    ["div.card-body .card-title > a", "h3.prod-name > a"];

const DEFAULT_IMAGE_URL_SELECTORS: [&str; 2] =
    ["figure.card-figure img.card-image", "a.prod-figure > img"];

const PRICE_WRAPPER: [&str; 3] = [
    "span[data-product-price-without-tax].price",
    "div.prod-price > span.price price--rrp",
    "div.prod-price > span.price",
];

const PRICE_WRAPPER_SALE: [&str; 2] = [
    "span[data-product-non-sale-price-without-tax].price",
    "div.prod-price > span.price price--sale",
];

pub(crate) struct BigCommerceBuilder {
    product_url_selector: Vec<String>,
    image_url_selector: Vec<String>,
    price_wrapper_selector: Vec<String>,
    price_wrapper_sale_selector: Vec<String>,
}

impl BigCommerceBuilder {
    pub(crate) fn default() -> Self {
        Self {
            product_url_selector: DEFAULT_PRODUCT_URL_SELECTORS
                .iter()
                .map(|selector| selector.to_string())
                .collect(),
            image_url_selector: DEFAULT_IMAGE_URL_SELECTORS
                .iter()
                .map(|selector| selector.to_string())
                .collect(),
            price_wrapper_selector: PRICE_WRAPPER
                .iter()
                .map(|selector| selector.to_string())
                .collect(),
            price_wrapper_sale_selector: PRICE_WRAPPER_SALE
                .iter()
                .map(|selector| selector.to_string())
                .collect(),
        }
    }

    pub(crate) fn with_product_url_selector(mut self, selector: impl Into<String>) -> Self {
        self.product_url_selector = vec![selector.into()];

        self
    }

    pub(crate) fn with_image_url_selector(mut self, selector: impl Into<String>) -> Self {
        self.image_url_selector = vec![selector.into()];

        self
    }

    pub(crate) fn with_price_wrapper_selector(mut self, selector: impl Into<String>) -> Self {
        self.price_wrapper_selector = vec![selector.into()];

        self
    }

    pub(crate) fn with_price_wrapper_sale_selector(mut self, selector: impl Into<String>) -> Self {
        self.price_wrapper_sale_selector = vec![selector.into()];

        self
    }

    pub(crate) fn build(self) -> BigCommerce {
        BigCommerce {
            options: self,
            parse_queue: Vec::new(),
        }
    }
}

pub(crate) struct BigCommerce {
    options: BigCommerceBuilder,
    pub(super) parse_queue: Vec<NestedProduct>,
}

impl BigCommerce {
    pub(super) fn parse_price(&self, element: ElementRef) -> Result<Price, RetailerError> {
        let main_price_element = match_element_from_list(
            element,
            &self.options.price_wrapper_selector,
            RetailerError::HtmlMissingElement("Missing valid normal price element".to_string()),
        )?;

        let main_price_text = element_to_text(main_price_element);

        let mut price = Price {
            regular_price: price_to_cents(main_price_text)?,
            sale_price: None,
        };

        if let Ok(non_sale_element) = match_element_from_list(
            element,
            &self.options.price_wrapper_sale_selector,
            RetailerError::HtmlMissingElement("Missing valid sale price element".to_string()),
        ) {
            // handle wolverine supplies combining english and price into same element
            // eg. "Was: $979.99"
            let text = element_to_text(non_sale_element);

            debug!("Unwrapping sale string for parsing: {text}");

            let non_sale_texts = text.split(" ");

            for chunk in non_sale_texts {
                match price_to_cents(chunk.to_string()) {
                    Ok(sale_price) => {
                        price.sale_price = Some(price.regular_price);
                        price.regular_price = sale_price;

                        break;
                    }
                    Err(_) => {}
                }
            }
        }

        Ok(price)
    }

    pub(crate) fn parse_max_pages(response: &String) -> Result<u64, RetailerError> {
        let html = Html::parse_document(response);

        let selector =
            Selector::parse("li:not(.pagination-item--next):not(.pagination-item--previous).pagination-item > a:not([rel])")
                .unwrap();

        let mut pagination_elements = html.select(&selector);

        let Some(last_page_element) = pagination_elements.next_back() else {
            return Ok(0);
        };

        let last_page_text = element_to_text(last_page_element);

        debug!("Page text: '{last_page_text}'");

        string_to_u64(last_page_text)
    }

    pub(super) fn get_image_url(&self, element: ElementRef) -> Result<String, RetailerError> {
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

    pub(super) fn get_title_element<'a>(
        &self,
        element: ElementRef<'a>,
    ) -> Result<ElementRef<'a>, RetailerError> {
        let link_element = match_element_from_list(
            element,
            &self.options.product_url_selector,
            RetailerError::HtmlMissingElement("Missing valid product link element".to_string()),
        )?;

        Ok(link_element)
    }

    pub(super) fn get_item_name(&self, element: ElementRef) -> Result<String, RetailerError> {
        let link_element = self.get_title_element(element)?;
        let product_name = element_to_text(link_element);

        Ok(product_name)
    }

    pub(super) fn get_item_link(&self, element: ElementRef) -> Result<String, RetailerError> {
        let link_element = self.get_title_element(element)?;
        let product_link = element_extract_attr(link_element, "href")?;

        Ok(product_link)
    }

    pub(crate) fn parse_product(
        &self,
        element: ElementRef,
        retailer: RetailerName,
        category: Category,
    ) -> Result<CrawlResult, RetailerError> {
        let image_url = clean_url(&self.get_image_url(element)?);
        let product_link = self.get_item_link(element)?;
        let product_name = self.get_item_name(element)?;

        let price = self.parse_price(element)?;

        let new_result = CrawlResult::new(product_name, product_link, price, retailer, category)
            .with_image_url(image_url);

        Ok(new_result)
    }
}
