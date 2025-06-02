use mongodb::bson::{Document, doc};

use super::traits::StageDocument;

pub(crate) struct DedupeStage;

impl DedupeStage {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl StageDocument for DedupeStage {
    fn get_stage_documents(&self) -> Vec<Document> {
        [doc! {
            "$group": {
                "_id": "$link",
                "doc": {
                    "$first": "$$ROOT"
                }
            }
        }]
        .into()
    }
}
