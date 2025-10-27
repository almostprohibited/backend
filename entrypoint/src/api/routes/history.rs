use std::{collections::BTreeMap, sync::Arc};

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use axum_extra::extract::WithRejection;
use chrono::{DateTime, NaiveTime};
use common::price_history::{ApiPriceHistoryInput, ApiPriceHistoryOutput, PriceHistoryEntry};
use tokio::time::Instant;
use tracing::debug;

use crate::{ServerState, routes::error_message_erasure::ApiError};

fn get_normalized_timestamp(timestamp: u64) -> u64 {
    // probably not an issue of stuffing unsigned into signed int
    // only cuts my max time in half to 292 billion years
    let normalized_timestamp = DateTime::from_timestamp(timestamp as i64, 0)
        .unwrap()
        .with_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
        .unwrap();

    normalized_timestamp.timestamp() as u64
}

fn get_lowest_price(price: &PriceHistoryEntry) -> u64 {
    price.sale_price.unwrap_or(price.regular_price)
}

// TODO: the results from this will parse and return the
// entire database of crawled results, meaning we'll
// potentially waste a bunch of processing parsing stuff
// outside the current max window which is currently 1 year back
pub(crate) async fn history_handler(
    State(state): State<Arc<ServerState>>,
    WithRejection(Query(query), _): WithRejection<Query<ApiPriceHistoryInput>, ApiError>,
) -> Result<impl IntoResponse, StatusCode> {
    let start_time: Instant = Instant::now();

    let Some(result) = state.db.get_pricing_history(query).await else {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    };

    let mut lowest_price: Option<PriceHistoryEntry> = None;
    let mut highest_price: Option<PriceHistoryEntry> = None;

    let mut history: BTreeMap<u64, PriceHistoryEntry> = BTreeMap::new();

    for price_entry in result.price_history {
        let normalized_timestamp = get_normalized_timestamp(price_entry.query_time);

        // perform this weird check since I probably won't remember to sort the output
        // from MongoDB, I only want the most recent crawl of the day
        if let Some(existing_entry) = history.get(&normalized_timestamp)
            && existing_entry.query_time < price_entry.query_time
        {
            // replace if newer
            history.insert(normalized_timestamp, price_entry.clone());
        } else {
            // insert if empty
            history.insert(normalized_timestamp, price_entry.clone());
        }

        let current_price = get_lowest_price(&price_entry);

        if let Some(ref unwrapped_lowest) = lowest_price {
            let current_lowest_price = get_lowest_price(unwrapped_lowest);

            if current_lowest_price > current_price
                || (current_lowest_price == current_price
                    && unwrapped_lowest.query_time > price_entry.query_time)
            {
                lowest_price = Some(price_entry.clone())
            }
        } else {
            lowest_price = Some(price_entry.clone())
        }

        if let Some(ref unwrapped_highest) = highest_price {
            let current_max_price = get_lowest_price(unwrapped_highest);

            if current_max_price < current_price
                || (current_max_price == current_price
                    && unwrapped_highest.query_time > price_entry.query_time)
            {
                highest_price = Some(price_entry.clone())
            }
        } else {
            highest_price = Some(price_entry.clone())
        }
    }

    debug!("Request time: {}ms", start_time.elapsed().as_millis());

    let response = Json::from(ApiPriceHistoryOutput {
        history: history
            .values()
            .cloned()
            .collect::<Vec<PriceHistoryEntry>>(),
        max_price: highest_price.expect("Max price to be populated"),
        min_price: lowest_price.expect("Min price to be populated"),
    });

    Ok(response.into_response())
}
