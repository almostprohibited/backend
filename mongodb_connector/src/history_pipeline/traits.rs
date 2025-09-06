use common::result::base::Price;
use mongodb::bson::{Document, doc, oid::ObjectId};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct HistoryParams {
    pub(crate) id: ObjectId,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HistoryResponse {
    pub price: Price,
    pub query_time: u64,
}

impl HistoryParams {
    pub(crate) fn find_product_document(&self) -> Document {
        doc! {
            "_id": &self.id
        }
    }

    pub(crate) fn get_all_documents(&self, name: String, url: String) -> Document {
        doc! {
            "name": name,
            "url": url,
        }
    }

    pub(crate) fn project_crawl_results(&self) -> Document {
        doc! {
            "price": 1,
            "query_time": 1,
            "_id": 0,
        }
    }
}
