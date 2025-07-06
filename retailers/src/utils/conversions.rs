use tracing::error;

use crate::errors::RetailerError;

/// Is capable of parsing the following into total cents:
/// 1. "$123.12"
/// 2. "123.12"
/// 3. "1,234.56"
///
/// Must have the cents in the original price
pub(crate) fn price_to_cents(price: String) -> Result<u64, RetailerError> {
    let mut trimmed_price = price.clone();

    if price.starts_with("$") {
        trimmed_price.remove(0);
    }

    trimmed_price = trimmed_price.replace(",", "");

    // lazily deal with missing cents
    // turns "100" -> "100.00"
    if !trimmed_price.contains(".") {
        trimmed_price = trimmed_price + ".00";
    }

    match trimmed_price.split_once(".") {
        Some((dollars, cents)) => {
            let parsed_dollars = string_to_u64(dollars.into())?;
            let parsed_cents = string_to_u64(cents.into())?;

            Ok(parsed_dollars * 100 + parsed_cents)
        }
        None => {
            error!("Failed to parse price, missing divider: {}", price);
            return Err(RetailerError::InvalidNumber(price));
        }
    }
}

pub(crate) fn string_to_u64(string: String) -> Result<u64, RetailerError> {
    let Ok(parsed_cents) = string.parse::<u64>() else {
        error!("Failed to parse string into u64 {}", string);
        return Err(RetailerError::InvalidNumber(string));
    };

    Ok(parsed_cents)
}
