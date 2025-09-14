use std::{cmp::min, collections::BTreeMap, sync::Arc};

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use axum_extra::extract::WithRejection;
use chrono::{DateTime, NaiveTime};
use mongodb_connector::history_pipeline::traits::HistoryParams;
use serde::Serialize;
use tokio::time::Instant;
use tracing::debug;

use crate::{ServerState, routes::error_message_erasure::ApiError};

const MAX_RESULTS: usize = 365;

#[derive(Serialize, Clone, Copy)]
struct FormattedHistory {
    // UNIX timestamp rounded to the nearest day
    normalized_timestamp: u64,
    price: Option<u64>,
}

#[derive(Serialize)]
struct OutputShape {
    history: Vec<FormattedHistory>,
    highest_price: FormattedHistory,
    lowest_price: FormattedHistory,
}

fn get_normalized_timestamp(timestamp: u64) -> u64 {
    // probably not an issue of stuffing unsigned into signed int
    // only cuts my max time in half to 292 billion years
    let normalized_timestamp = DateTime::from_timestamp(timestamp as i64, 0)
        .unwrap()
        .with_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
        .unwrap();

    normalized_timestamp.timestamp() as u64
}

// TODO: the results from this will parse the entire database
// of crawled results, meaning we'll potentially waste a bunch
// of processing parsing stuff outside the current max window
// which is currently 1 year back
pub(crate) async fn history_handler(
    State(state): State<Arc<ServerState>>,
    WithRejection(Query(query), _): WithRejection<Query<HistoryParams>, ApiError>,
) -> Result<impl IntoResponse, StatusCode> {
    let start_time: Instant = Instant::now();

    let result = state.db.get_pricing_history(query).await;

    if result.is_empty() {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    }

    let mut lowest_price: Option<FormattedHistory> = None;
    let mut highest_price: Option<FormattedHistory> = None;

    let mut history: BTreeMap<u64, FormattedHistory> = BTreeMap::new();

    for unformatted_result in result {
        let price = unformatted_result
            .price
            .sale_price
            .unwrap_or(unformatted_result.price.regular_price);

        let normalized_timestamp = get_normalized_timestamp(unformatted_result.query_time);

        // create lowest price, accounting for sales
        let formatted_history = FormattedHistory {
            normalized_timestamp,
            price: Some(price),
        };

        // deal with dedupes (several crawls that might have accidentially happened)
        if let Some(existing_history) = history.get_mut(&normalized_timestamp)
            && existing_history.price.is_some()
        {
            // safe unwrap as optional condition checked above
            let existing_price = existing_history.price.unwrap();

            if existing_price > price {
                existing_history.price = Some(price);
            }
        } else {
            history.insert(normalized_timestamp, formatted_history);
        };

        if let Some(ref lowest) = lowest_price {
            if (lowest.price > formatted_history.price)
                || (lowest.price == formatted_history.price
                    && lowest.normalized_timestamp > formatted_history.normalized_timestamp)
            {
                lowest_price = Some(formatted_history);
            }
        } else {
            lowest_price = Some(formatted_history);
        };

        if let Some(ref highest) = highest_price {
            if (highest.price < formatted_history.price)
                || (highest.price == formatted_history.price
                    && highest.normalized_timestamp > formatted_history.normalized_timestamp)
            {
                highest_price = Some(formatted_history);
            }
        } else {
            highest_price = Some(formatted_history);
        };
    }

    // insert blanks into response
    let mut current_timestamp = match history.last_entry() {
        Some(last_value) => *last_value.key(),
        None => 0,
    };

    let first_timestamp = match history.first_entry() {
        Some(first_value) => *first_value.key(),
        None => 0,
    };

    while current_timestamp > first_timestamp {
        history
            .entry(current_timestamp)
            .or_insert(FormattedHistory {
                normalized_timestamp: current_timestamp,
                price: None,
            });

        // rewind one day
        current_timestamp -= 24 * 60 * 60;
    }

    // note: drain causes memory leak if iterator leaves scope
    let final_vec = history
        .values()
        .copied()
        .collect::<Vec<FormattedHistory>>()
        .drain(history.len() - min(MAX_RESULTS, history.len())..)
        .collect();

    let response: Json<OutputShape> = Json::from(OutputShape {
        history: final_vec,
        lowest_price: lowest_price.expect("Expect there to be a lowest price"),
        highest_price: highest_price.expect("Expect there to be a highest price"),
    });

    debug!("Request time: {}ms", start_time.elapsed().as_millis());

    Ok(response.into_response())
}
