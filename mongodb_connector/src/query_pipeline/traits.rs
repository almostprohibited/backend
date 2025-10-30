use common::search_params::ApiSearchInput;
use mongodb::bson::Document;
use mongodb::bson::doc;
use tracing::trace;

use super::match_stage::MatchStage;
use super::page_stage::PageStage;
use super::sort_stage::SortStage;

pub(crate) struct SearchPipeline {
    search_query: ApiSearchInput,
}

impl SearchPipeline {
    pub(crate) fn new(search_query: ApiSearchInput) -> Self {
        Self { search_query }
    }

    pub(crate) fn get_search_documents(&self) -> Vec<Document> {
        let mut documents: Vec<Document> = Vec::new();

        documents.extend(MatchStage::new(self.search_query.clone()).get_stage_documents());
        documents.extend(vec![
            doc! {
                "$group": {
                    "_id": {
                        "url": "$url",
                        "name": "$name",
                    },
                    "doc": {
                        "$first": "$$ROOT"
                    }
                }
            },
            doc! {
                "$replaceRoot": {
                    "newRoot": "$doc"
                }
            },
        ]);
        documents.extend(SortStage::new(self.search_query.clone()).get_stage_documents());
        documents.push(doc! {
            "$facet": {
                "items": PageStage::new(self.search_query.clone()).get_stage_documents(),
                "total_count": [
                    {
                        "$count": "count"
                    }
                ]
            }
        });

        trace!("Documents: {:#?}", documents);

        documents
    }
}

pub(crate) trait StageDocument {
    fn get_stage_documents(&self) -> Vec<Document>;
}
