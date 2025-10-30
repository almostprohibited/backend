use common::{
    result::base::CrawlResult,
    search_params::{ApiSearchInput, CollectionSearchResults},
};
use mongodb::{
    Client, Collection, Database, IndexModel,
    bson::{doc, oid::ObjectId},
    options::IndexOptions,
};
use serde::Deserialize;
use tracing::debug;

use crate::{
    constants::{DATABASE_NAME, VIEW_LIVE_DATA_NAME, VIEW_LIVE_DATA_SEARCH_INDEX},
    query_pipeline::traits::SearchPipeline,
};

#[derive(Deserialize)]
struct PaginatedSearchOutput {
    items: Vec<CrawlResult>,
    total_count: Vec<PaginatedCountOutput>,
}

impl PaginatedSearchOutput {
    fn get_count(&self) -> u64 {
        let Some(count_obj) = self.total_count.first() else {
            return 0;
        };

        count_obj.count
    }
}

#[derive(Deserialize)]
struct PaginatedCountOutput {
    count: u64,
}

pub(crate) struct LiveResultsView {
    collection: Collection<CrawlResult>,
}

impl LiveResultsView {
    pub(crate) async fn new(client: Client) -> Self {
        let db = client.database(DATABASE_NAME);

        Self::create_collection(&db).await;

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

    pub(crate) async fn search_items(
        &self,
        query_params: &ApiSearchInput,
    ) -> CollectionSearchResults {
        let pipeline_documents = SearchPipeline::new(query_params.clone()).get_search_documents();

        debug!("Using: {:?}", pipeline_documents);

        let mut cursor = self
            .collection
            .aggregate(pipeline_documents)
            .with_type::<PaginatedSearchOutput>()
            .await
            .unwrap();

        let mut result = CollectionSearchResults::new();

        while cursor.advance().await.unwrap_or(false) {
            let paginated_result = cursor.deserialize_current().unwrap();

            result.total_count += paginated_result.get_count();
            result.items.extend(paginated_result.items);
        }

        result
    }

    pub(crate) async fn prune_results(&self, prev_days: i64) {
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
