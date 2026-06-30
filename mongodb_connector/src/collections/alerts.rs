use common::{
    db::{NotificationChannel, ServiceType, VerificationStatus},
    utils::get_current_time,
};
use mongodb::{
    Client, Collection, Database,
    bson::{doc, oid::ObjectId},
};

use crate::constants::{COLLECTION_NOTIFICATION_CHANNELS_NAME, DATABASE_NAME};

pub(crate) struct AlertCollection {
    notification_channel_collection: Collection<NotificationChannel>,
}

impl AlertCollection {
    pub(crate) async fn new(client: Client) -> Self {
        let db = client.database(DATABASE_NAME);

        Self::create_collection(&db).await;

        Self {
            notification_channel_collection: db
                .collection::<NotificationChannel>(COLLECTION_NOTIFICATION_CHANNELS_NAME),
        }
    }

    async fn create_collection(db: &Database) {
        db.create_collection(COLLECTION_NOTIFICATION_CHANNELS_NAME)
            .await
            .unwrap_or_else(|_| {
                panic!("Creating {COLLECTION_NOTIFICATION_CHANNELS_NAME} collection to not fail")
            });
    }

    pub(crate) async fn create_notification_channel(
        &self,
        identifier: &str,
        user_id: ObjectId,
        service: ServiceType,
        status: VerificationStatus,
    ) -> bool {
        let timestamp = get_current_time();

        self.notification_channel_collection
            .insert_one(NotificationChannel {
                user_id,
                service_type: service,
                identifier: identifier.to_string(),
                created_at: timestamp,
                last_updated_at: timestamp,
                status: status,
            })
            .await
            .is_ok()
    }

    pub(crate) async fn get_notification_channels(
        &self,
        user_id: ObjectId,
    ) -> Vec<NotificationChannel> {
        let mut cursor = self
            .notification_channel_collection
            .find(doc! {
                "user_id": user_id,
            })
            .await
            .unwrap();

        let mut result: Vec<NotificationChannel> = vec![];

        while cursor.advance().await.unwrap_or(false) {
            result.push(cursor.deserialize_current().unwrap());
        }

        result
    }

    /// Deletes multiple notification channels.
    /// If `identifier` is not passed, delete all channels
    pub(crate) async fn delete_notification_channels(
        &self,
        user_id: ObjectId,
        identifier: Option<&str>,
    ) -> bool {
        let mut query = doc! {
            "user_id": user_id,
        };

        if let Some(identifier) = identifier {
            query.insert("identifier", identifier);
        }

        let result = self
            .notification_channel_collection
            .delete_many(query)
            .await
            .unwrap();

        result.deleted_count > 0
    }
}
