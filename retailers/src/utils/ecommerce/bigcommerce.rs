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

pub(crate) struct BigCommerce {}

impl BigCommerce {
    pub(crate) fn parse_price(element: ElementRef) -> Result<Price, RetailerError> {
        let main_price_element = extract_element_from_element(
            element,
            "div.price-section.price-section--withoutTax.current-price > span.price",
        )?;
        let main_price_text = element_to_text(main_price_element);

        let mut price = Price {
            regular_price: price_to_cents(main_price_text)?,
            sale_price: None,
        };

        if let Ok(non_sale_element) = extract_element_from_element(
            element,
            "div.price-section.price-section--withoutTax.non-sale-price > span.price",
        ) {
            price.sale_price = Some(price.regular_price);

            let non_sale_text = element_to_text(non_sale_element);
            price.regular_price = price_to_cents(non_sale_text)?;
        }

        Ok(price)
    }

    pub(crate) fn parse_max_pages(response: &String) -> Result<u64, RetailerError> {
        let html = Html::parse_document(response);

        let selector =
            Selector::parse("li:not(.pagination-item--next):not(.pagination-item--previous).pagination-item > a.pagination-link")
                .unwrap();

        let mut pagination_elements = html.select(&selector);

        let Some(last_page_element) = pagination_elements.next_back() else {
            return Ok(0);
        };

        let last_page_text = element_to_text(last_page_element);

        string_to_u64(last_page_text)
    }

    pub(crate) fn get_image_url(element: ElementRef) -> Result<String, RetailerError> {
        let image_element =
            extract_element_from_element(element, "figure.card-figure img.card-image")?;

        if let Ok(data_src) = element_extract_attr(image_element, "data-src")
            && data_src.starts_with("https")
        {
            return Ok(data_src);
        };

        if let Ok(regular_src) = element_extract_attr(image_element, "src")
            && regular_src.starts_with("https")
        {
            return Ok(regular_src);
        }

        Err(RetailerError::HtmlElementMissingAttribute(
            "'valid data-src or src'".into(),
            element_to_text(image_element),
        ))
    }

    fn get_title_element(element: ElementRef) -> Result<ElementRef, RetailerError> {
        let details_body_element = extract_element_from_element(element, "div.card-body")?;
        let link_element = extract_element_from_element(details_body_element, "h4.card-title > a")?;

        Ok(link_element)
    }

    pub(crate) fn get_item_name(element: ElementRef) -> Result<String, RetailerError> {
        let link_element = Self::get_title_element(element)?;
        let product_name = element_to_text(link_element);

        Ok(product_name)
    }

    pub(crate) fn get_item_link(element: ElementRef) -> Result<String, RetailerError> {
        let link_element = Self::get_title_element(element)?;
        let product_link = element_extract_attr(link_element, "href")?;

        Ok(product_link)
    }

    pub(crate) fn parse_product(
        element: ElementRef,
        retailer: RetailerName,
        category: Category,
    ) -> Result<CrawlResult, RetailerError> {
        let image_url = Self::get_image_url(element)?;

        let details_body_element = extract_element_from_element(element, "div.card-body")?;

        let product_link = Self::get_item_link(element)?;
        let product_name = Self::get_item_name(element)?;

        let price = Self::parse_price(details_body_element)?;

        let new_result = CrawlResult::new(product_name, product_link, price, retailer, category)
            .with_image_url(image_url);

        Ok(new_result)
    }
}
