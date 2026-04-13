use common::result::{
    base::{CrawlResult, Price},
    enums::RetailerName,
};
use scraper::{ElementRef, Html, Selector};

use crate::{
    errors::RetailerError,
    structures::HtmlSearchQuery,
    utils::{
        conversions::{price_to_cents, string_to_u64},
        helpers::clean_url,
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

pub(crate) struct Magento {}

impl Magento {
    fn parse_prices(element: ElementRef) -> Result<Price, RetailerError> {
        let final_price_el = extract_element_from_element(
            element,
            "span.price-wrapper[data-price-type=finalPrice] > span",
        )?;
        let final_price = price_to_cents(element_to_text(final_price_el))?;

        let mut price = Price {
            regular_price: final_price,
            sale_price: None,
        };

        if let Ok(old_price_element) = extract_element_from_element(
            element,
            "span.price-wrapper[data-price-type=oldPrice] > span",
        ) {
            price.regular_price = price_to_cents(element_to_text(old_price_element))?;
            price.sale_price = Some(final_price);
        };

        Ok(price)
    }

    fn parse_image_url(element: ElementRef) -> Result<String, RetailerError> {
        let image_element =
            extract_element_from_element(element, "a.product-item-photo img.product-image-photo")?;

        let raw_image_url = element_extract_attr(image_element, "src")?;

        Ok(clean_url(&raw_image_url))
    }

    pub(crate) fn is_valid_product(element: ElementRef) -> Result<bool, RetailerError> {
        let details_element = extract_element_from_element(element, "div.product-item-details")?;
        let link_element = extract_element_from_element(details_element, "a.product-item-link")?;

        if let Ok(data_bind_attr) = element_extract_attr(link_element, "data-bind")
            && !data_bind_attr.is_empty()
        {
            return Ok(false);
        }

        return Ok(true);
    }

    pub(crate) fn parse_product(
        element: ElementRef,
        retailer: RetailerName,
        search_term: &HtmlSearchQuery,
    ) -> Result<CrawlResult, RetailerError> {
        let details_element = extract_element_from_element(element, "div.product-item-details")?;
        let link_element = extract_element_from_element(details_element, "a.product-item-link")?;

        let url = element_extract_attr(link_element, "href")?;
        let name = element_to_text(link_element);
        let price = Self::parse_prices(details_element)?;

        let new_result = CrawlResult::new(name, url, price, retailer, search_term.category)
            .with_image_url(Self::parse_image_url(element)?);

        Ok(new_result)
    }

    pub(crate) fn get_pages(response: &str) -> Result<u64, RetailerError> {
        let fragment = Html::parse_document(response);
        let page_number_selector =
            Selector::parse("ul.pages-items > li.item > a.page > span:not(.label)").unwrap();

        let mut page_links = fragment.select(&page_number_selector);

        let Some(last_page_element) = page_links.next_back() else {
            return Ok(0);
        };

        string_to_u64(element_to_text(last_page_element))
    }
}
