use mongodb::{Client, Collection, IndexModel, bson::doc};
use retailers::results::firearm::FirearmResult;

use crate::{stages::traits::QueryParams, structs::Count};

const CONNECTION_URI: &str = "mongodb://root:root@localhost:27017";
const DATABASE_NAME: &str = "project-carbon";
const COLLECTION_FIREARMS_NAME: &str = "firearms";

pub struct MongoDBConnector {
    // mongodb structs are already Arc, thread safe
    firearms_collection: Collection<FirearmResult>,
}

impl MongoDBConnector {
    pub async fn new() -> Self {
        let client = Client::with_uri_str(CONNECTION_URI).await.unwrap();

        Self::initialize(client.clone()).await;

        Self {
            firearms_collection: client
                .database(DATABASE_NAME)
                .collection::<FirearmResult>(COLLECTION_FIREARMS_NAME),
        }
    }

    async fn initialize(client: Client) {
        let db = client.database(DATABASE_NAME);

        let _ = db
            .create_collection(COLLECTION_FIREARMS_NAME)
            .await
            .unwrap();

        let index = IndexModel::builder()
            .keys(doc! {
                "name": "text"
            })
            .build();

        let _ = db
            .collection::<FirearmResult>(COLLECTION_FIREARMS_NAME)
            .create_index(index)
            .await
            .unwrap();
    }

    pub async fn search(&self, query_params: &QueryParams) -> Vec<FirearmResult> {
        let mut cursor = self
            .firearms_collection
            .aggregate(query_params.get_search_documents())
            .with_type::<FirearmResult>()
            .await
            .unwrap();

        let mut result: Vec<FirearmResult> = Vec::new();

        while cursor.advance().await.unwrap() {
            result.push(cursor.deserialize_current().unwrap());
        }

        result
    }

    pub async fn count(&self, query_params: &QueryParams) -> Count {
        let cursor = self
            .firearms_collection
            .aggregate(query_params.get_count_documents())
            .with_type::<Count>()
            .await
            .unwrap();

        cursor.deserialize_current().unwrap()
    }

    //db.firearms.aggregate({"$match":{"$text":{"$search":"sks"}}}, {"$group":{"_id":"$link","doc":{"$first":"$$ROOT"}}}, {"$sort":{"score":{"$meta":"textScore"}}}, {"$replaceRoot":{"newRoot":"$doc"}})

    pub async fn insert_many_firearms(&self, firearms: Vec<FirearmResult>) {
        self.firearms_collection
            .insert_many(firearms)
            .await
            .unwrap();
    }
}
