use std::{collections::HashMap, sync::Arc};

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use axum_extra::extract::WithRejection;
use mongodb_connector::history_pipeline::traits::HistoryParams;
use serde::Serialize;
use tokio::time::Instant;
use tracing::debug;

use crate::{ServerState, routes::error_message_erasure::ApiError};

#[derive(Serialize, Clone, Copy)]
struct FormattedHistory {
    // UNIX timestamp rounded to the nearest day
    normalized_timestamp: u64,
    price: u64,
}

#[derive(Serialize)]
struct OutputShape {
    history: Vec<FormattedHistory>,
    highest_price: FormattedHistory,
    lowest_price: FormattedHistory,
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

    let mut history: HashMap<u64, FormattedHistory> = HashMap::new();

    for unformatted_result in result {
        let price = match unformatted_result.price.sale_price {
            Some(sale_price) => sale_price,
            None => unformatted_result.price.regular_price,
        };

        let normalized_timestamp = (unformatted_result.query_time / 3600) * 3600;

        let formatted_history = FormattedHistory {
            normalized_timestamp,
            price,
        };

        // deal with dedupes (several crawls that might have accidentially happened)
        if let Some(existing_history) = history.get_mut(&normalized_timestamp) {
            if existing_history.price < price {
                existing_history.price = price;
            }
        } else {
            history.insert(normalized_timestamp, formatted_history.clone());
        };

        if let Some(ref lowest) = lowest_price {
            if (lowest.price > formatted_history.price)
                || (lowest.price == formatted_history.price
                    && lowest.normalized_timestamp > formatted_history.normalized_timestamp)
            {
                lowest_price = Some(formatted_history.clone());
            }
        } else {
            lowest_price = Some(formatted_history.clone());
        };

        if let Some(ref highest) = highest_price {
            if (highest.price < formatted_history.price)
                || (highest.price == formatted_history.price
                    && highest.normalized_timestamp > formatted_history.normalized_timestamp)
            {
                highest_price = Some(formatted_history.clone());
            }
        } else {
            highest_price = Some(formatted_history);
        };
    }

    let response: Json<OutputShape> = Json::from(OutputShape {
        history: history.values().copied().collect(),
        lowest_price: lowest_price.expect("Expect there to be a lowest price"),
        highest_price: highest_price.expect("Expect there to be a highest price"),
    });

    debug!("Request time: {}ms", start_time.elapsed().as_millis());

    Ok(response.into_response())
}
