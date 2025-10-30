use common::result::base::CrawlResult;
use mongodb::{Client, Collection, Database, bson::doc};

use crate::constants::{COLLECTION_CRAWL_RESULTS_NAME, DATABASE_NAME, VIEW_LIVE_DATA_NAME};

pub(crate) struct CrawlResultsCollection {
    collection: Collection<CrawlResult>,
}

impl CrawlResultsCollection {
    pub(crate) async fn new(client: Client) -> Self {
        let db = client.database(DATABASE_NAME);

        Self::create_collection(&db).await;

        Self {
            collection: db.collection::<CrawlResult>(COLLECTION_CRAWL_RESULTS_NAME),
        }
    }

    async fn create_collection(db: &Database) {
        db.create_collection(COLLECTION_CRAWL_RESULTS_NAME)
            .await
            .unwrap_or_else(|_| {
                panic!("Creating {COLLECTION_CRAWL_RESULTS_NAME} collection to not fail")
            });
    }

    pub(crate) async fn insert_results(&self, results: Vec<&CrawlResult>) {
        self.collection.insert_many(results).await.unwrap();
    }

    pub(crate) async fn update_view(&self, prev_days: i64) {
        self.collection
            .aggregate(vec![
                doc! {"$match": {"query_time": {"$gte": prev_days}}},
                doc! {"$merge": {"into": VIEW_LIVE_DATA_NAME, "whenMatched": "keepExisting", "on": "_id"}},
            ])
            .with_type::<CrawlResult>()
            .await
            .unwrap();
    }
}
