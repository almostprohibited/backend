use mongodb::{Client, Collection, IndexModel, bson::doc};
use retailers::results::firearm::FirearmResult;
use tracing::info;

const CONNECTION_URI: &str = "mongodb://root:root@localhost:27017";
const DATABASE_NAME: &str = "project-carbon";
const COLLECTION_FIREARMS_NAME: &str = "firearms";

pub struct MongoDBConnector {
    // mongodb client is already Arc, thread safe
    client: Client,
    firearms_collection: Collection<FirearmResult>,
}

impl MongoDBConnector {
    pub async fn new() -> Self {
        let client = Client::with_uri_str(CONNECTION_URI).await.unwrap();

        Self::initialize(client.clone()).await;

        Self {
            client: client.clone(),
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

    pub async fn search(&self, search_string: impl Into<String>) -> Vec<FirearmResult> {
        let string_obj: String = search_string.into();
        let search_terms = string_obj
            .split(" ")
            .map(|term| format!("\"{}\"", term))
            .collect::<Vec<String>>()
            .join(" ");

        let mut cursor = self
            .firearms_collection
            .aggregate([
                doc! {
                    "$match": {
                        "$text": {
                            "$search": search_terms
                        }
                    }
                },
                doc! {
                    "$group": {
                        "_id": "$link",
                        "doc": {
                            "$first": "$$ROOT"
                        }
                    }
                },
                doc! {
                    "$sort": {
                        "score": {
                            "$meta": "textScore"
                        }
                    }
                },
                doc! {
                    "$replaceRoot": {
                        "newRoot": "$doc"
                    }
                },
            ])
            .with_type::<FirearmResult>()
            .await
            .unwrap();

        // let mut cursor = self
        //     .firearms_collection
        //     .find(doc! {
        //         "$text": {
        //             "$search": search_terms
        //         }
        //     })
        //     .sort(doc! {
        //         "score": {
        //             "$meta": "textScore"
        //         }
        //     })
        //     .await
        //     .unwrap();

        let mut result: Vec<FirearmResult> = Vec::new();

        while cursor.advance().await.unwrap() {
            result.push(cursor.deserialize_current().unwrap());
        }

        result
    }

    pub async fn insert_many_firearms(&self, firearms: Vec<FirearmResult>) {
        self.firearms_collection
            .insert_many(firearms)
            .await
            .unwrap();
    }
}
