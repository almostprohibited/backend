use common::{
    price_history::{CollectionPriceHistory, PriceHistoryEntry},
    result::base::CrawlResult,
};
use mongodb::{
    Client, Collection, Database, IndexModel,
    bson::{doc, to_bson},
    options::IndexOptions,
};

use crate::constants::{COLLECTION_PRICE_HISTORY_NAME, DATABASE_NAME};

const INDEX_NAME: &str = "search_index";

pub(crate) struct PriceHistoryCollection {
    collection: Collection<CollectionPriceHistory>,
}

impl PriceHistoryCollection {
    pub(crate) async fn new(client: Client) -> Self {
        let db = client.database(DATABASE_NAME);

        Self::create_collection(&db).await;

        Self {
            collection: db.collection::<CollectionPriceHistory>(COLLECTION_PRICE_HISTORY_NAME),
        }
    }

    async fn create_collection(db: &Database) {
        db.create_collection(COLLECTION_PRICE_HISTORY_NAME)
            .await
            .unwrap_or_else(|_| {
                panic!("Creating {COLLECTION_PRICE_HISTORY_NAME} collection to not fail")
            });

        let index = IndexModel::builder()
            .keys(doc! {
                "name": "text",
                "url": "text"
            })
            .options(IndexOptions::builder().name(INDEX_NAME.to_string()).build())
            .build();

        db.collection::<CollectionPriceHistory>(COLLECTION_PRICE_HISTORY_NAME)
            .create_index(index)
            .await
            .unwrap();
    }

    pub(crate) async fn get_price_history(
        &self,
        name: impl Into<String>,
        url: impl Into<String>,
    ) -> CollectionPriceHistory {
        self.collection
            .find_one(doc! {
                "name": name.into(),
                "url": url.into()
            })
            .await
            .unwrap_or_else(|_| {
                panic!("find_one call to not fail for {COLLECTION_PRICE_HISTORY_NAME}")
            })
            .expect("find_one call to actually find something")
    }

    pub(crate) async fn update_collection(&self, results: Vec<&CrawlResult>) {
        for result in results {
            let price_obj = PriceHistoryEntry {
                regular_price: result.price.regular_price,
                sale_price: result.price.sale_price,
                query_time: result.query_time,
            };

            let parsed_price =
                to_bson(&price_obj).expect("PriceHistoryEntry to deserialize correctly");

            let Ok(update_result) = self
                .collection
                .update_one(
                    doc! {
                        "name": result.name.clone(),
                        "url": result.url.clone()
                    },
                    doc! {
                        "$push": doc! {
                            "price_history": parsed_price
                        }
                    },
                )
                .await
            else {
                // TODO: this normally shouldn't fail, but handle this better
                continue;
            };

            if update_result.matched_count == 0 {
                let _ = self.collection.insert_one(CollectionPriceHistory {
                    name: result.name.clone(),
                    url: result.url.clone(),
                    price_history: vec![price_obj],
                });
            }
        }
    }
}
