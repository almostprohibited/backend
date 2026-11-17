use chrono::Utc;
use common::{
    result::{
        base::CrawlResult,
        enums::{Category, RetailerName},
        metadata::{Ammunition, Metadata},
    },
    retailer_status::{RetailerProductCountsBuilder, RetailerStatus},
};
use mongodb::{Client, Collection, Database, IndexModel, bson::doc, options::IndexOptions};

use crate::constants::{COLLECTION_RETAILER_STATUS_NAME, DATABASE_NAME};

const GET_INDEX_NAME: &str = "get_index";

pub(crate) struct RetailerStatusCollection {
    collection: Collection<RetailerStatus>,
}

impl RetailerStatusCollection {
    pub(crate) async fn new(client: Client) -> Self {
        let db = client.database(DATABASE_NAME);

        Self::create_collection(&db).await;

        Self {
            collection: db.collection::<RetailerStatus>(COLLECTION_RETAILER_STATUS_NAME),
        }
    }

    async fn create_collection(db: &Database) {
        db.create_collection(COLLECTION_RETAILER_STATUS_NAME)
            .await
            .unwrap_or_else(|_| {
                panic!("Creating {COLLECTION_RETAILER_STATUS_NAME} collection to not fail")
            });

        let get_index = IndexModel::builder()
            .keys(doc! {
                "retailer": 1,
                "timestamp": -1
            })
            .options(
                IndexOptions::builder()
                    .name(GET_INDEX_NAME.to_string())
                    .build(),
            )
            .build();

        db.collection::<RetailerStatus>(COLLECTION_RETAILER_STATUS_NAME)
            .create_index(get_index)
            .await
            .unwrap();
    }

    pub(crate) async fn insert_status(
        &self,
        retailer_name: RetailerName,
        successful: bool,
        products: &Vec<&CrawlResult>,
    ) {
        let mut firearms: u64 = 0;
        let mut other: u64 = 0;
        let mut ammo: u64 = 0;
        let mut ammo_no_count: u64 = 0;

        products.iter().for_each(|product| match product.category {
            Category::Firearm => firearms += 1,
            Category::Ammunition => {
                if let Some(Metadata::Ammunition(Ammunition {
                    round_count: Some(_),
                    ..
                })) = product.metadata
                {
                    ammo += 1;
                } else {
                    ammo_no_count += 1;
                }
            }
            Category::Other => other += 1,
            _ => {}
        });

        let counts = RetailerProductCountsBuilder::new()
            .with_firearms(firearms)
            .with_ammo(ammo)
            .with_ammo_no_count(ammo_no_count)
            .with_other(other)
            .build();

        let status = RetailerStatus {
            retailer: retailer_name,
            counts: counts,
            successful,
            timestamp: Utc::now(),
        };

        let _ = self.collection.insert_one(status).await.unwrap();
    }

    pub(crate) async fn get_recent_status(&self) -> Vec<RetailerStatus> {
        let mut cursor = self
            .collection
            .aggregate([
                doc! {
                    "$sort": {"retailer": 1, "timestamp": -1}
                },
                doc! {
                    "$group": {"_id": "$retailer", "doc": { "$first": "$$ROOT" } }
                },
                doc! {
                    "$replaceRoot": {"newRoot": "$doc" }
                },
            ])
            .with_type::<RetailerStatus>()
            .await
            .unwrap();

        let mut results: Vec<RetailerStatus> = vec![];

        while cursor.advance().await.unwrap_or(false) {
            let result = cursor.deserialize_current().unwrap();

            results.push(result);
        }

        results
    }
}
