use scraper::{ElementRef, Selector};
use tracing::error;

use crate::errors::RetailerError;

pub(crate) fn element_to_text(element: ElementRef) -> String {
    element.text().collect::<String>().trim().into()
}

pub(crate) fn element_extract_attr(
    element: ElementRef,
    attr_name: String,
) -> Result<String, RetailerError> {
    let Some(attr_value) = element.attr(&attr_name) else {
        error!(
            "Failed to find attribute {} in element {:?}",
            attr_name, element
        );
        return Err(RetailerError::HtmlElementMissingAttribute(
            attr_name,
            element.html(),
        ));
    };

    Ok(attr_value.to_string().trim().into())
}

pub(crate) fn extract_element_from_element(
    element: ElementRef,
    query_string: String,
) -> Result<ElementRef, RetailerError> {
    let selector = Selector::parse(&query_string).unwrap();

    let Some(query_element) = element.select(&selector).next() else {
        error!(
            "Failed to find element '{}' in parent element {:?}",
            query_string, element
        );

        return Err(RetailerError::HtmlMissingElement(query_string));
    };

    Ok(query_element)
}
