use common::{messages::Message, result::base::CrawlResult};
use mongodb::{Client, Collection, IndexModel, bson::doc};
use tracing::debug;

use crate::{stages::traits::QueryParams, structs::Count};

const CONNECTION_URI: &str = "mongodb://root:root@localhost:27017";
const DATABASE_NAME: &str = "project-carbon";
const COLLECTION_CRAWL_RESULTS_NAME: &str = "crawl-results";
const COLLECTION_MESSAGES_NAME: &str = "messages";

pub struct MongoDBConnector {
    // mongodb structs are already Arc, thread safe
    crawl_results_collection: Collection<CrawlResult>,
    messages_collection: Collection<Message>,
}

impl MongoDBConnector {
    pub async fn new() -> Self {
        let client = Client::with_uri_str(CONNECTION_URI).await.unwrap();

        Self::initialize(client.clone()).await;

        Self {
            crawl_results_collection: client
                .database(DATABASE_NAME)
                .collection::<CrawlResult>(COLLECTION_CRAWL_RESULTS_NAME),
            messages_collection: client
                .database(DATABASE_NAME)
                .collection::<Message>(COLLECTION_MESSAGES_NAME),
        }
    }

    async fn initialize(client: Client) {
        let db = client.database(DATABASE_NAME);

        let _ = db
            .create_collection(COLLECTION_CRAWL_RESULTS_NAME)
            .await
            .unwrap();

        let _ = db
            .create_collection(COLLECTION_MESSAGES_NAME)
            .await
            .unwrap();

        let crawl_result_search_index = IndexModel::builder()
            .keys(doc! {
                "name": "text"
            })
            .build();

        let _ = db
            .collection::<CrawlResult>(COLLECTION_CRAWL_RESULTS_NAME)
            .create_index(crawl_result_search_index)
            .await
            .unwrap();
    }

    pub async fn insert_message(&self, message: Message) {
        let _ = self.messages_collection.insert_one(message).await.unwrap();
    }

    pub async fn search_items(&self, query_params: &QueryParams) -> Vec<CrawlResult> {
        let mut cursor = self
            .crawl_results_collection
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
            .crawl_results_collection
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

    pub async fn insert_many_results(&self, results: Vec<CrawlResult>) {
        self.crawl_results_collection
            .insert_many(results)
            .await
            .unwrap();
    }
}
