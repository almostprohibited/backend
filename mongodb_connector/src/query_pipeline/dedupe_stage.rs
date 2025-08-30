use mongodb::bson::{Document, doc};

use super::traits::StageDocument;

pub(super) struct DedupeStage;

impl DedupeStage {
    pub(super) fn new() -> Self {
        Self {}
    }
}

impl StageDocument for DedupeStage {
    fn get_stage_documents(&self) -> Vec<Document> {
        [doc! {
            "$group": {
                "_id": {
                    "url": "$url",
                    "name": "$name",
                },
                "doc": {
                    "$first": "$$ROOT"
                }
            }
        }]
        .into()
    }
}
