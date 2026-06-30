use std::{env, str::FromStr, sync::LazyLock};

use common::{
    db::{AuthVerification, NotificationChannel, ServiceType, Session, VerificationStatus},
    messages::Message,
    price_history::{ApiPriceHistoryInput, CollectionPriceHistory},
    result::base::CrawlResult,
    search_params::{ApiSearchInput, CollectionSearchResults},
    string_utils::sha256_hash_string,
};
use mongodb::{Client, bson::oid::ObjectId};
use tracing::{debug, warn};

use crate::collections::{
    alerts::AlertCollection, authentication::AuthenticationCollections,
    crawl_results::CrawlResultsCollection, live_results::LiveResultsView,
    messages::MessagesCollection, price_history::PriceHistoryCollection,
};

static CONNECTION_URI: LazyLock<String> = LazyLock::new(|| {
    let host = env::var("MONGO_DB_HOST").unwrap_or("localhost".into());
    let port = env::var("MONGO_DB_PORT").unwrap_or("27017".into());

    format!("mongodb://root:root@{host}:{port}")
});

pub struct MongoDBConnector {
    crawl_results: CrawlResultsCollection,
    live_results: LiveResultsView,
    messages: MessagesCollection,
    price_history: PriceHistoryCollection,
    authentication: AuthenticationCollections,
    alerts: AlertCollection,
}

impl MongoDBConnector {
    pub async fn new() -> Self {
        let client = Client::with_uri_str(CONNECTION_URI.to_string())
            .await
            .unwrap();

        Self {
            crawl_results: CrawlResultsCollection::new(client.clone()).await,
            live_results: LiveResultsView::new(client.clone()).await,
            messages: MessagesCollection::new(client.clone()).await,
            price_history: PriceHistoryCollection::new(client.clone()).await,
            authentication: AuthenticationCollections::new(client.clone()).await,
            alerts: AlertCollection::new(client).await,
        }
    }

    pub async fn insert_message(&self, message: Message) {
        self.messages.insert_message(message).await;
    }

    pub async fn search_items(&self, query_params: &ApiSearchInput) -> CollectionSearchResults {
        self.live_results.search_items(query_params).await
    }

    pub async fn insert_many_results(&self, results: Vec<&CrawlResult>) {
        tokio::join!(
            self.crawl_results.insert_results(results.clone()),
            self.live_results.insert_results(results.clone()),
            self.price_history.update_collection(results)
        );
    }

    pub async fn get_pricing_history(
        &self,
        query: ApiPriceHistoryInput,
    ) -> Option<CollectionPriceHistory> {
        let Some(result) = self.live_results.find_result(query.id).await else {
            warn!("Invalid ID, no results found for {}", query.id.to_string());

            return None;
        };

        Some(
            self.price_history
                .get_price_history(result.name, result.url)
                .await,
        )
    }

    pub async fn find_result(&self, object_id: String) -> Option<CrawlResult> {
        let Ok(id) = ObjectId::from_str(&object_id) else {
            return None;
        };

        self.live_results.find_result(id).await
    }

    pub async fn create_session(
        &self,
        unhashed_service_identifier: &str,
        service_type: ServiceType,
        unhashed_token: &str,
        created_at: u64,
        ip_addr: &str,
    ) {
        let hashed_service_identifier = sha256_hash_string(unhashed_service_identifier);

        let user_object = match self
            .authentication
            .find_user_by_identifier(service_type.clone(), &hashed_service_identifier)
            .await
        {
            Some(user) => user.id.expect("Expect ID of existing user to be populated"),
            None => {
                self.authentication
                    .create_user(service_type.clone(), &hashed_service_identifier)
                    .await
            }
        };

        self.authentication
            .create_session(Session {
                user_id: user_object,
                service_type,
                hashed_token: sha256_hash_string(unhashed_token),
                created_at,
                ip_addr: ip_addr.to_string(),
            })
            .await;
    }

    pub async fn find_session(&self, unhashed_token: &str) -> Option<Session> {
        self.authentication
            .find_session(&sha256_hash_string(unhashed_token))
            .await
    }

    pub async fn delete_session(&self, unhashed_token: &str) -> bool {
        self.authentication
            .delete_session(&sha256_hash_string(unhashed_token))
            .await
    }

    // no cascade deletes with document based stores, no regrets
    pub async fn nuke_account(&self, unhashed_token: &str) -> bool {
        if cfg!(debug_assertions) {
            debug!("{unhashed_token}\n{}", sha256_hash_string(unhashed_token));
        }

        let Some(user) = self
            .authentication
            .find_session(&sha256_hash_string(unhashed_token))
            .await
        else {
            debug!("Failed to find session");

            return false;
        };

        let (session_deleted, user_deleted, alerts_deleted) = tokio::join!(
            self.authentication.delete_sessions_by_user_id(user.user_id),
            self.authentication.delete_user(user.user_id),
            self.alerts.delete_notification_channels(user.user_id, None),
        );

        debug!("{session_deleted} && {user_deleted} && {alerts_deleted}");

        session_deleted && user_deleted && alerts_deleted
    }

    pub async fn create_verification(
        &self,
        unhashed_code: &str,
        timestamp: u64,
        ip_addr: Option<String>,
        nonce: Option<String>,
    ) {
        self.authentication
            .create_verification(AuthVerification {
                hashed_code: sha256_hash_string(unhashed_code),
                timestamp,
                ip_addr,
                nonce,
            })
            .await;
    }

    pub async fn get_verification(&self, unhashed_code: &str) -> Option<AuthVerification> {
        self.authentication
            .get_verification(&sha256_hash_string(unhashed_code))
            .await
    }

    pub async fn delete_verification(&self, unhashed_code: &str) -> bool {
        self.authentication
            .delete_verification(&sha256_hash_string(unhashed_code))
            .await
    }

    /// Gets all ammo documents within live results table
    pub async fn get_all_live_ammo(&self) -> Vec<CrawlResult> {
        self.live_results.get_all_live_ammo().await
    }

    // TODO: I should probably check for existing entries
    /// Method does not check or dedupe entries
    pub async fn create_notification_channel(
        &self,
        identifier: &str,
        user_id: ObjectId,
        service: ServiceType,
        status: VerificationStatus,
    ) {
        let _ = self
            .alerts
            .create_notification_channel(identifier, user_id, service, status)
            .await;
    }

    pub async fn get_notification_channels(&self, user_id: ObjectId) -> Vec<NotificationChannel> {
        self.alerts.get_notification_channels(user_id).await
    }

    pub async fn delete_notification_channel(&self, user_id: ObjectId, identifier: &str) -> bool {
        self.alerts
            .delete_notification_channels(user_id, Some(identifier))
            .await
    }
}
