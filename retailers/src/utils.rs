use scraper::ElementRef;
use tracing::error;

use crate::errors::RetailerError;

/// Is capable of parsing the following into total cents:
/// 1. "$123.12"
/// 2. "123.12"
/// 3. "1,234.56"
///
/// Must have the cents in the original price
pub(crate) fn price_to_cents(price: String) -> Result<u32, RetailerError> {
    let mut trimmed_price = price.clone();

    if price.starts_with("$") {
        trimmed_price.remove(0);
    }

    trimmed_price = trimmed_price.replace(",", "");

    match trimmed_price.split_once(".") {
        Some((dollars, cents)) => {
            let Ok(parsed_dollars) = dollars.parse::<u32>() else {
                error!("Failed to parse dollar amount: {} ({})", dollars, price);
                return Err(RetailerError::InvalidPrice(price));
            };

            let Ok(parsed_cents) = cents.parse::<u32>() else {
                error!("Failed to parse cent amount: {} ({})", dollars, price);
                return Err(RetailerError::InvalidPrice(price));
            };

            Ok(parsed_dollars * 100 + parsed_cents)
        }
        None => {
            error!("Failed to parse price, missing divider: {}", price);
            return Err(RetailerError::InvalidPrice(price));
        }
    }
}

pub(crate) fn element_to_text(element: ElementRef) -> String {
    element.text().collect::<String>().trim().to_string()
}
