use std::{env, sync::LazyLock};

use common::{messages::Message, result::base::CrawlResult, utils::normalized_relative_days};
use mongodb::{Client, Collection, IndexModel, bson::doc, options::IndexOptions};
use tracing::{debug, warn};

use crate::{
    history_pipeline::traits::{HistoryParams, HistoryResponse},
    query_pipeline::traits::QueryParams,
    structs::Count,
};

const CONNECTION_URI: LazyLock<String> = LazyLock::new(|| {
    let host = env::var("MONGO_DB_HOST").unwrap_or("localhost".into());
    let port = env::var("MONGO_DB_PORT").unwrap_or("27017".into());

    format!("mongodb://root:root@{host}:{port}")
});

const DATABASE_NAME: &str = "project-carbon";
const COLLECTION_CRAWL_RESULTS_NAME: &str = "crawl-results";
const COLLECTION_MESSAGES_NAME: &str = "messages";

const VIEW_LIVE_DATA_NAME: &str = "live-results";

// TODO: I should have assigned a name to the index
// when I created this thing, todo to refactor this
const SEARCH_INDEX_NAME: &str = "name_text";
// const SEARCH_INDEX_NAME: &str = "base_search_index";

pub struct MongoDBConnector {
    // mongodb structs are already Arc, thread safe
    crawl_results: Collection<CrawlResult>,
    live_results: Collection<CrawlResult>,
    messages: Collection<Message>,
}

impl MongoDBConnector {
    pub async fn new() -> Self {
        let client = Client::with_uri_str(CONNECTION_URI.to_string())
            .await
            .unwrap();

        Self::initialize(client.clone()).await;

        Self {
            crawl_results: client
                .database(DATABASE_NAME)
                .collection::<CrawlResult>(COLLECTION_CRAWL_RESULTS_NAME),
            live_results: client
                .database(DATABASE_NAME)
                .collection::<CrawlResult>(VIEW_LIVE_DATA_NAME),
            messages: client
                .database(DATABASE_NAME)
                .collection::<Message>(COLLECTION_MESSAGES_NAME),
        }
    }

    async fn initialize(client: Client) {
        let db = client.database(DATABASE_NAME);

        db.create_collection(COLLECTION_CRAWL_RESULTS_NAME)
            .await
            .expect("Creating crawl results collection to not fail");

        db.create_collection(VIEW_LIVE_DATA_NAME)
            .await
            .expect("Creating live results collection to not fail");

        db.create_collection(COLLECTION_MESSAGES_NAME)
            .await
            .expect("Creating messages collection to not fail");

        let crawl_result_search_index = IndexModel::builder()
            .keys(doc! {
                "name": "text",
            })
            .options(
                IndexOptions::builder()
                    .name(SEARCH_INDEX_NAME.to_string())
                    .build(),
            )
            .build();

        db.collection::<CrawlResult>(COLLECTION_CRAWL_RESULTS_NAME)
            .create_index(crawl_result_search_index.clone())
            .await
            .unwrap();

        db.collection::<CrawlResult>(VIEW_LIVE_DATA_NAME)
            .create_index(crawl_result_search_index)
            .await
            .unwrap();
    }

    pub async fn insert_message(&self, message: Message) {
        let _ = self.messages.insert_one(message).await.unwrap();
    }

    pub async fn search_items(&self, query_params: &QueryParams) -> Vec<CrawlResult> {
        let mut cursor = self
            .live_results
            .aggregate(query_params.get_search_documents())
            .with_type::<CrawlResult>()
            .await
            .unwrap();

        debug!("Using: {:?}", query_params.get_search_documents());

        let mut result: Vec<CrawlResult> = Vec::new();

        while cursor.advance().await.unwrap_or(false) {
            result.push(cursor.deserialize_current().unwrap());
        }

        result
    }

    pub async fn count_items(&self, query_params: &QueryParams) -> Count {
        let mut cursor = self
            .live_results
            .aggregate(query_params.get_count_documents())
            .with_type::<Count>()
            .await
            .unwrap();

        let Ok(has_count) = cursor.advance().await else {
            return Count { total_count: 0 };
        };

        match has_count {
            true => cursor.deserialize_current().unwrap(),
            false => Count { total_count: 0 },
        }
    }

    pub async fn insert_many_results(&self, results: Vec<&CrawlResult>) {
        self.crawl_results.insert_many(results).await.unwrap();

        self.update_view().await;
    }

    pub async fn update_view(&self) {
        let prev_days = normalized_relative_days(3);

        self.live_results
            .delete_many(doc! {
                "query_time": {"$lt": prev_days}
            })
            .await
            .unwrap();

        self.crawl_results
            .aggregate(vec![
                doc! {"$match": {"query_time": {"$gte": prev_days}}},
                doc! {"$merge": {"into": VIEW_LIVE_DATA_NAME, "whenMatched": "keepExisting", "on": "_id"}},
            ])
            .with_type::<CrawlResult>()
            .await
            .unwrap();
    }

    pub async fn get_pricing_history(&self, query: HistoryParams) -> Vec<HistoryResponse> {
        let Some(result) = self
            .live_results
            .find_one(query.find_product_document())
            .await
            .unwrap()
        else {
            warn!("Invalid ID, no results found for {}", query.id.to_string());

            return vec![];
        };

        let mut documents = self
            .crawl_results
            .aggregate([
                doc! {
                    "$match": query.get_all_documents(result.name, result.url)
                },
                doc! {"$project": query.project_crawl_results()},
            ])
            .with_type::<HistoryResponse>()
            .await
            .unwrap();

        let mut results: Vec<HistoryResponse> = Vec::new();

        while documents.advance().await.unwrap_or(false) {
            results.push(documents.deserialize_current().unwrap());
        }

        results
    }
}
