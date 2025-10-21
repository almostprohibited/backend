use common::messages::Message;
use mongodb::{Client, Collection, Database};

use crate::constants::{COLLECTION_MESSAGES_NAME, DATABASE_NAME};

pub(crate) struct MessagesCollection {
    collection: Collection<Message>,
}

impl MessagesCollection {
    pub(crate) async fn new(client: Client) -> Self {
        let db = client.database(DATABASE_NAME);

        Self::create_collection(&db).await;

        Self {
            collection: db.collection::<Message>(COLLECTION_MESSAGES_NAME),
        }
    }

    async fn create_collection(db: &Database) {
        db.create_collection(COLLECTION_MESSAGES_NAME)
            .await
            .expect(&format!(
                "Creating {COLLECTION_MESSAGES_NAME} collection to not fail"
            ));
    }

    pub(crate) async fn insert_message(&self, message: Message) {
        let _ = self.collection.insert_one(message).await.unwrap();
    }
}
