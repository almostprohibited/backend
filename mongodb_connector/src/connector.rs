use std::{env, str::FromStr, sync::LazyLock};

use common::{
    AuthVerification,
    messages::Message,
    price_history::{ApiPriceHistoryInput, CollectionPriceHistory},
    result::base::CrawlResult,
    search_params::{ApiSearchInput, CollectionSearchResults},
    string_utils::sha256_hash_string,
    user_sessions::{ServiceType, Session},
    utils::normalized_relative_days,
};
use mongodb::{Client, bson::oid::ObjectId};
use tracing::warn;

use crate::collections::{
    authentication::AuthenticationCollections, crawl_results::CrawlResultsCollection,
    live_results::LiveResultsView, messages::MessagesCollection,
    price_history::PriceHistoryCollection,
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
            authentication: AuthenticationCollections::new(client).await,
        }
    }

    pub async fn insert_message(&self, message: Message) {
        self.messages.insert_message(message).await;
    }

    pub async fn search_items(&self, query_params: &ApiSearchInput) -> CollectionSearchResults {
        self.live_results.search_items(query_params).await
    }

    pub async fn insert_many_results(&self, results: Vec<&CrawlResult>) {
        self.crawl_results.insert_results(results.clone()).await;

        let prev_days = normalized_relative_days(3);

        self.live_results.prune_results(prev_days).await;
        self.crawl_results.update_view(prev_days).await;
        self.price_history.update_collection(results).await;
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

    pub async fn find_session(&self, hashed_token: &str) -> Option<Session> {
        self.authentication.find_session(hashed_token).await
    }

    pub async fn delete_session(&self, unhashed_token: &str) -> bool {
        self.authentication
            .delete_session(&sha256_hash_string(unhashed_token))
            .await
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
}
