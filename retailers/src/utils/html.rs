use scraper::{ElementRef, Selector};

use crate::errors::RetailerError;

pub(crate) fn element_to_text(element: ElementRef) -> String {
    element.text().collect::<String>().trim().into()
}

pub(crate) fn element_extract_attr(
    element: ElementRef,
    attr_name: impl Into<String>,
) -> Result<String, RetailerError> {
    let attribute: String = attr_name.into();

    let Some(attr_value) = element.attr(&attribute) else {
        return Err(RetailerError::HtmlElementMissingAttribute(
            attribute,
            element.html(),
        ));
    };

    Ok(attr_value.to_string().trim().into())
}

pub(crate) fn extract_element_from_element(
    element: ElementRef,
    query_string: impl Into<String>,
) -> Result<ElementRef, RetailerError> {
    let query: String = query_string.into();

    let selector = Selector::parse(&query).unwrap();

    let Some(query_element) = element.select(&selector).next() else {
        return Err(RetailerError::HtmlMissingElement(query));
    };

    Ok(query_element)
}
