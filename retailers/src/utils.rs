use scraper::ElementRef;

/// Is capable of parsing the following into total cents:
/// 1. "$123.12"
/// 2. "123.12"
/// 3. "1,234.56"
///
/// Must have the cents in the original price
pub(crate) fn price_to_cents(price: String) -> u32 {
    let mut trimmed_price = price.clone();

    if price.starts_with("$") {
        trimmed_price.remove(0);
    }

    trimmed_price = trimmed_price.replace(",", "");

    match trimmed_price.split_once(".") {
        Some((dollars, cents)) => {
            let parsed_dollars = dollars.parse::<u32>().unwrap();
            let parsed_cents = cents.parse::<u32>().unwrap();

            parsed_dollars * 100 + parsed_cents
        }
        None => 0,
    }
}

pub(crate) fn element_to_text(element: ElementRef) -> String {
    element.text().collect::<String>().trim().to_string()
}
