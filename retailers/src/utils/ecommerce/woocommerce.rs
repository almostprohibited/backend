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

pub(crate) struct WooCommerceBuilder {
    product_name_selector: String,
    product_url_selector: String,
    image_url_selector: String,
}

impl WooCommerceBuilder {
    pub(crate) fn default() -> Self {
        Self {
            product_name_selector: "div.product-element-bottom > h3 > a".into(),
            product_url_selector: "div.product-element-bottom > h3 > a".into(),
            image_url_selector: "a.product-image-link > img".into(),
        }
    }

    pub(crate) fn with_product_name_selector(mut self, selector: impl Into<String>) -> Self {
        self.product_name_selector = selector.into();

        self
    }

    pub(crate) fn with_product_url_selector(mut self, selector: impl Into<String>) -> Self {
        self.product_url_selector = selector.into();

        self
    }

    pub(crate) fn with_image_url_selector(mut self, selector: impl Into<String>) -> Self {
        self.image_url_selector = selector.into();

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
        let image_element =
            extract_element_from_element(element, self.options.image_url_selector.clone())?;

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

        Err(RetailerError::HtmlElementMissingAttribute(
            "'valid data-src or src'".into(),
            element_to_text(image_element),
        ))
    }

    pub(crate) fn parse_product(
        &self,
        element: ElementRef,
        retailer: RetailerName,
        category: Category,
    ) -> Result<CrawlResult, RetailerError> {
        let url_element =
            extract_element_from_element(element, self.options.product_url_selector.clone())?;
        let name_element =
            extract_element_from_element(element, self.options.product_name_selector.clone())?;

        let name = element_to_text(name_element);
        let url = element_extract_attr(url_element, "href")?;

        let image_url = self.get_image_url(element)?;

        let new_product =
            CrawlResult::new(name, url, Self::parse_price(element)?, retailer, category)
                .with_image_url(image_url);

        Ok(new_product)
    }
}
