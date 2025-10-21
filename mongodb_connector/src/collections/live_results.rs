use common::result::base::CrawlResult;
use mongodb::{
    Client, Collection, Database, IndexModel,
    bson::{doc, oid::ObjectId},
    options::IndexOptions,
};
use tracing::debug;

use crate::{
    constants::{DATABASE_NAME, VIEW_LIVE_DATA_NAME, VIEW_LIVE_DATA_SEARCH_INDEX},
    query_pipeline::traits::QueryParams,
    structs::Count,
};

pub(crate) struct LiveResultsView {
    collection: Collection<CrawlResult>,
}

impl LiveResultsView {
    pub(crate) async fn new(client: Client) -> Self {
        let db = client.database(DATABASE_NAME);

        Self::create_collection(&db).await;
        Self::create_indexes(&db).await;

        Self {
            collection: db.collection::<CrawlResult>(VIEW_LIVE_DATA_NAME),
        }
    }

    async fn create_collection(db: &Database) {
        db.create_collection(VIEW_LIVE_DATA_NAME)
            .await
            .expect(&format!(
                "Creating {VIEW_LIVE_DATA_NAME} collection to not fail"
            ));
    }

    async fn create_indexes(db: &Database) {
        let crawl_result_search_index = IndexModel::builder()
            .keys(doc! {
                "name": "text",
            })
            .options(
                IndexOptions::builder()
                    .name(VIEW_LIVE_DATA_SEARCH_INDEX.to_string())
                    .build(),
            )
            .build();

        db.collection::<CrawlResult>(VIEW_LIVE_DATA_NAME)
            .create_index(crawl_result_search_index)
            .await
            .unwrap();
    }

    pub(crate) async fn search_items(&self, query_params: &QueryParams) -> Vec<CrawlResult> {
        let mut cursor = self
            .collection
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

    pub(crate) async fn count_items(&self, query_params: &QueryParams) -> Count {
        let mut cursor = self
            .collection
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

    pub(crate) async fn update_view(&self, prev_days: i64) {
        self.collection
            .delete_many(doc! {
                "query_time": {"$lt": prev_days}
            })
            .await
            .unwrap();
    }

    pub(crate) async fn find_result(&self, object_id: ObjectId) -> Option<CrawlResult> {
        self.collection
            .find_one(doc! {
                "_id": object_id
            })
            .await
            .unwrap()
    }
}
