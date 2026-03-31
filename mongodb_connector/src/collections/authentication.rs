use std::time::Duration;

use common::{
    constants::TOKEN_COOKIE_TTL_SECS,
    user_sessions::{ServiceIdentifier, ServiceType, Session, User},
};
use mongodb::{
    Client, Collection, Database, IndexModel,
    bson::{doc, oid::ObjectId},
    options::IndexOptions,
};

use crate::constants::{
    COLLECTION_SESSIONS_NAME, COLLECTION_USERS_NAME, DATABASE_NAME, SESSIONS_EXPIRE_INDEX,
};

pub(crate) struct AuthenticationCollections {
    users_collection: Collection<User>,
    sessions_collection: Collection<Session>,
}

impl AuthenticationCollections {
    pub(crate) async fn new(client: Client) -> Self {
        let db = client.database(DATABASE_NAME);

        Self::create_collection(&db).await;

        Self {
            users_collection: db.collection::<User>(COLLECTION_USERS_NAME),
            sessions_collection: db.collection::<Session>(COLLECTION_SESSIONS_NAME),
        }
    }

    async fn create_collection(db: &Database) {
        db.create_collection(COLLECTION_USERS_NAME)
            .await
            .unwrap_or_else(|_| panic!("Creating {COLLECTION_USERS_NAME} collection to not fail"));

        db.create_collection(COLLECTION_SESSIONS_NAME)
            .await
            .unwrap_or_else(|_| {
                panic!("Creating {COLLECTION_SESSIONS_NAME} collection to not fail")
            });

        let ttl_index = IndexModel::builder()
            .keys(doc! {
                "created_at": 1,
            })
            .options(
                IndexOptions::builder()
                    .name(SESSIONS_EXPIRE_INDEX.to_string())
                    // offset TTL by a minute in case something weird happens
                    // where client lives longer than we do
                    .expire_after(Some(Duration::from_secs(TOKEN_COOKIE_TTL_SECS + 60)))
                    .build(),
            )
            .build();

        db.collection::<Session>(COLLECTION_SESSIONS_NAME)
            .create_index(ttl_index)
            .await
            .unwrap();
    }

    pub(crate) async fn find_user(&self, user_id: ObjectId) -> Option<User> {
        self.users_collection
            .find_one(doc! {"_id": user_id})
            .await
            .unwrap()
    }

    pub(crate) async fn find_user_by_identifier(
        &self,
        provider: ServiceType,
        identifier: &str,
    ) -> Option<User> {
        self.users_collection
            .find_one(doc! {
                "linked_services": ServiceIdentifier {
                    service_type: provider,
                    identifier: identifier.to_string()
                }
            })
            .await
            .unwrap()
    }

    pub(crate) async fn create_user(&self, provider: ServiceType, identifier: &str) -> ObjectId {
        self.users_collection
            .insert_one(User {
                linked_services: vec![ServiceIdentifier {
                    service_type: provider,
                    identifier: identifier.to_string(),
                }],
                id: None,
            })
            .await
            .unwrap()
            .inserted_id
            .as_object_id()
            .unwrap()
    }

    pub(crate) async fn create_session(&self, session: Session) {
        self.sessions_collection
            .insert_one(session)
            .await
            .unwrap()
            .inserted_id;
    }

    pub(crate) async fn find_session(&self, hashed_token: &str) -> Option<Session> {
        self.sessions_collection
            .find_one(doc! {
                "hashed_token": hashed_token
            })
            .await
            .unwrap()
    }

    pub(crate) async fn delete_session(&self, hashed_token: &str) -> bool {
        let result = self
            .sessions_collection
            .delete_one(doc! {
                "hashed_token": hashed_token
            })
            .await
            .unwrap();

        result.deleted_count > 0
    }
}
