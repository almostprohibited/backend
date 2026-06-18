use crate::serde_utils::{from_datetime_to_u64_seconds, from_u64_seconds_to_datetime};
use mongodb::bson::{Bson, doc, oid::ObjectId};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ServiceType {
    Email,
    Discord,
    Google,
    Microsoft,
}

impl Into<Bson> for ServiceType {
    fn into(self) -> Bson {
        Bson::String(format!("{self:?}").to_lowercase())
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ServiceIdentifier {
    pub service_type: ServiceType,
    pub hashed_identifier: String,
}

impl Into<Bson> for ServiceIdentifier {
    fn into(self) -> Bson {
        Bson::Document(doc! {
            "service_type": self.service_type,
            "hashed_identifier": self.hashed_identifier
        })
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct User {
    #[serde(rename(deserialize = "_id"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub linked_services: Vec<ServiceIdentifier>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Session {
    pub user_id: ObjectId,
    pub service_type: ServiceType,
    pub hashed_token: String,
    #[serde(serialize_with = "from_u64_seconds_to_datetime")]
    #[serde(deserialize_with = "from_datetime_to_u64_seconds")]
    pub created_at: u64,
    pub ip_addr: String,
}
