use mongodb::{Client, Collection, IndexModel, bson::doc};
use retailers::results::firearm::FirearmResult;

const CONNECTION_URI: &str = "mongodb://root:root@localhost:27017";
const DATABASE_NAME: &str = "project-carbon";
const COLLECTION_FIREARMS_NAME: &str = "firearms";

pub struct MongoDBConnector {
    // mongodb client is already Arc, thread safe
    client: Client,
}

impl MongoDBConnector {
    pub async fn new() -> Self {
        let client = Client::with_uri_str(CONNECTION_URI).await.unwrap();

        Self::initialize(client.clone()).await;

        Self { client }
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

    //db.firearms.find({$text: {$search: "\"sks\""}}).sort({score: {$meta: "textScore"}})

    pub async fn insert_many_firearms(&self, firearms: Vec<FirearmResult>) {
        let db: Collection<FirearmResult> = self
            .client
            .database(DATABASE_NAME)
            .collection::<FirearmResult>(COLLECTION_FIREARMS_NAME);

        db.insert_many(firearms).await.unwrap();
    }
}
