use common::result::base::CrawlResult;
use mongodb::{Client, Collection, Database, IndexModel, bson::doc, options::IndexOptions};

use crate::{
    constants::{
        COLLECTION_CRAWL_RESULTS_HISTORY_INDEX, COLLECTION_CRAWL_RESULTS_NAME, DATABASE_NAME,
        VIEW_LIVE_DATA_NAME,
    },
    history_pipeline::traits::{HistoryParams, HistoryResponse},
};

pub(crate) struct CrawlResultsCollection {
    collection: Collection<CrawlResult>,
}

impl CrawlResultsCollection {
    pub(crate) async fn new(client: Client) -> Self {
        let db = client.database(DATABASE_NAME);

        Self::create_collection(&db).await;
        Self::create_indexes(&db).await;

        Self {
            collection: db.collection::<CrawlResult>(COLLECTION_CRAWL_RESULTS_NAME),
        }
    }

    async fn create_collection(db: &Database) {
        db.create_collection(COLLECTION_CRAWL_RESULTS_NAME)
            .await
            .expect(&format!(
                "Creating {COLLECTION_CRAWL_RESULTS_NAME} collection to not fail"
            ));
    }

    async fn create_indexes(db: &Database) {
        let history_index = IndexModel::builder()
            .keys(doc! {
                "name": "text",
                "url": "text",
            })
            .options(
                IndexOptions::builder()
                    .name(COLLECTION_CRAWL_RESULTS_HISTORY_INDEX.to_string())
                    .build(),
            )
            .build();

        db.collection::<CrawlResult>(COLLECTION_CRAWL_RESULTS_NAME)
            .create_index(history_index)
            .await
            .unwrap();
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

    pub(crate) async fn get_price_history(
        &self,
        search_item: CrawlResult,
        history_query: HistoryParams,
    ) -> Vec<HistoryResponse> {
        let mut documents = self
            .collection
            .aggregate([
                doc! {
                    "$match": history_query.get_all_documents(search_item.name, search_item.url)
                },
                doc! {"$project": history_query.project_crawl_results()},
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
