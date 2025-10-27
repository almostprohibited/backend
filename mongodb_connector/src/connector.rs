use std::{env, sync::LazyLock};

use common::{
    messages::Message,
    price_history::{ApiPriceHistoryInput, CollectionPriceHistory},
    result::base::CrawlResult,
    utils::normalized_relative_days,
};
use mongodb::Client;
use tracing::warn;

use crate::{
    collections::{
        crawl_results::CrawlResultsCollection, live_results::LiveResultsView,
        messages::MessagesCollection, price_history::PriceHistoryCollection,
    },
    query_pipeline::traits::QueryParams,
    structs::Count,
};

const CONNECTION_URI: LazyLock<String> = LazyLock::new(|| {
    let host = env::var("MONGO_DB_HOST").unwrap_or("localhost".into());
    let port = env::var("MONGO_DB_PORT").unwrap_or("27017".into());

    format!("mongodb://root:root@{host}:{port}")
});

pub struct MongoDBConnector {
    crawl_results: CrawlResultsCollection,
    live_results: LiveResultsView,
    messages: MessagesCollection,
    price_history: PriceHistoryCollection,
}

impl MongoDBConnector {
    pub async fn new() -> Self {
        let client = Client::with_uri_str(CONNECTION_URI.to_string())
            .await
            .unwrap();

        Self {
            crawl_results: CrawlResultsCollection::new(client.clone()).await,
            live_results: LiveResultsView::new(client.clone()).await,
            messages: MessagesCollection::new(client.clone()).await,
            price_history: PriceHistoryCollection::new(client).await,
        }
    }

    pub async fn insert_message(&self, message: Message) {
        self.messages.insert_message(message).await;
    }

    pub async fn search_items(&self, query_params: &QueryParams) -> Vec<CrawlResult> {
        self.live_results.search_items(query_params).await
    }

    pub async fn count_items(&self, query_params: &QueryParams) -> Count {
        self.live_results.count_items(query_params).await
    }

    pub async fn insert_many_results(&self, results: Vec<&CrawlResult>) {
        self.crawl_results.insert_results(results.clone()).await;

        let prev_days = normalized_relative_days(3);

        self.live_results.prune_results(prev_days).await;
        self.crawl_results.update_view(prev_days).await;
        self.price_history.update_collection(results).await;
    }

    pub async fn get_pricing_history(
        &self,
        query: ApiPriceHistoryInput,
    ) -> Option<CollectionPriceHistory> {
        let Some(result) = self.live_results.find_result(query.id).await else {
            warn!("Invalid ID, no results found for {}", query.id.to_string());

            return None;
        };

        Some(
            self.price_history
                .get_price_history(result.name, result.url)
                .await,
        )
    }
}
