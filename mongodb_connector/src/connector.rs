use common::{messages::Message, result::base::CrawlResult};
use mongodb::{Client, Collection, IndexModel, bson::doc};

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

    // pub async fn search_firearms(&self, query_params: &QueryParams) -> Vec<Firearm> {
    //     let mut cursor = self
    //         .firearms_collection
    //         .aggregate(query_params.get_search_documents())
    //         .with_type::<Firearm>()
    //         .await
    //         .unwrap();

    //     let mut result: Vec<Firearm> = Vec::new();

    //     while cursor.advance().await.unwrap() {
    //         result.push(cursor.deserialize_current().unwrap());
    //     }

    //     result
    // }

    // pub async fn count_firearms(&self, query_params: &QueryParams) -> Count {
    //     let cursor = self
    //         .firearms_collection
    //         .aggregate(query_params.get_count_documents())
    //         .with_type::<Count>()
    //         .await
    //         .unwrap();

    //     cursor.deserialize_current().unwrap()
    // }

    pub async fn insert_many_results(&self, results: Vec<CrawlResult>) {
        self.crawl_results_collection
            .insert_many(results)
            .await
            .unwrap();
    }
}
