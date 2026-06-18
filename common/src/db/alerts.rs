use crate::serde_utils::{from_datetime_to_u64_seconds, from_u64_seconds_to_datetime};

use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

use crate::db::ServiceType;

#[derive(Deserialize, Clone, Serialize, Debug)]
pub struct NotificationChannel {
    pub user_id: ObjectId,
    pub service_type: ServiceType,
    // TODO: encrypt this
    pub identifier: String,
    #[serde(serialize_with = "from_u64_seconds_to_datetime")]
    #[serde(deserialize_with = "from_datetime_to_u64_seconds")]
    pub created_at: u64,
    #[serde(serialize_with = "from_u64_seconds_to_datetime")]
    #[serde(deserialize_with = "from_datetime_to_u64_seconds")]
    pub last_updated_at: u64,
    pub status: VerificationStatus,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum VerificationStatus {
    Verified,
    Pending,
}
