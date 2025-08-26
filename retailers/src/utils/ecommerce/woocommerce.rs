use common::result::{
    base::{CrawlResult, Price},
    enums::{Category, RetailerName},
};
use scraper::{ElementRef, Html, Selector};

use crate::{
    errors::RetailerError,
    utils::{
        conversions::{price_to_cents, string_to_u64},
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

pub(crate) struct WooCommerce {}

impl WooCommerce {
    pub(crate) fn parse_price(element: ElementRef) -> Result<Price, RetailerError> {
        let mut price = Price {
            regular_price: 0,
            sale_price: None,
        };

        let regular_non_sale_price =
            extract_element_from_element(element, "span.price > span.amount > bdi");

        match regular_non_sale_price {
            Ok(regular_price_element) => {
                price.regular_price = price_to_cents(element_to_text(regular_price_element))?;
            }
            Err(_) => {
                let sale_price =
                    extract_element_from_element(element, "span.price > ins > span.amount > bdi")?;
                let previous_price =
                    extract_element_from_element(element, "span.price > del > span.amount > bdi")?;

                price.regular_price = price_to_cents(element_to_text(previous_price))?;
                price.sale_price = Some(price_to_cents(element_to_text(sale_price))?);
            }
        }

        Ok(price)
    }

    pub(crate) fn parse_max_pages(response: &String) -> Result<u64, RetailerError> {
        let fragment = Html::parse_document(&response);
        let page_number_selector =
            Selector::parse("ul.page-numbers > li > a:not(.next):not(.prev).page-numbers").unwrap();

        let page_links = fragment.select(&page_number_selector);

        let Some(last_page_element) = page_links.last() else {
            return Ok(0);
        };

        Ok(string_to_u64(element_to_text(last_page_element))?)
    }

    fn get_image_url(element: ElementRef) -> Result<String, RetailerError> {
        let valid_selectors = vec![
            "a.product-image-link > img",
            "a.woocommerce-LoopProduct-link > img", // international shooting supplies specific
        ];

        let Some(image_element) = valid_selectors.iter().find_map(|selector| {
            if let Ok(extracted_element) = extract_element_from_element(element, *selector) {
                return Some(extracted_element);
            }

            None
        }) else {
            return Err(RetailerError::HtmlMissingElement(
                "product image element".into(),
            ));
        };

        if let Ok(data_src) = element_extract_attr(image_element, "data-src")
            && data_src.starts_with("https")
            && !data_src.contains("lazy")
        {
            return Ok(data_src);
        };

        if let Ok(regular_src) = element_extract_attr(image_element, "src")
            && regular_src.starts_with("https")
            && !regular_src.contains("lazy")
        {
            return Ok(regular_src);
        }

        return Err(RetailerError::HtmlElementMissingAttribute(
            "'valid data-src or src'".into(),
            element_to_text(image_element),
        ));
    }

    pub(crate) fn parse_product(
        element: ElementRef,
        retailer: RetailerName,
        category: Category,
    ) -> Result<CrawlResult, RetailerError> {
        let valid_selectors = vec![
            "div.product-element-bottom > h3 > a",
            "div.astra-shop-summary-wrap > a.ast-loop-product__link", // international shooting supplies specific
        ];

        let Some(title_element) = valid_selectors.iter().find_map(|selector| {
            if let Ok(extracted_element) = extract_element_from_element(element, *selector) {
                return Some(extracted_element);
            }

            None
        }) else {
            return Err(RetailerError::HtmlMissingElement(
                "product title element".into(),
            ));
        };

        let name = element_to_text(title_element);
        let url = element_extract_attr(title_element, "href")?;

        let image_url = Self::get_image_url(element)?;

        let new_product =
            CrawlResult::new(name, url, Self::parse_price(element)?, retailer, category)
                .with_image_url(image_url);

        Ok(new_product)
    }
}
