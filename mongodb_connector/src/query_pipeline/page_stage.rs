use common::search_params::ApiSearchInput;
use mongodb::bson::{Document, doc};

use super::traits::StageDocument;

const MAX_ITEMS_PER_PAGE: u32 = 32;

pub(super) struct PageStage {
    search_query: ApiSearchInput,
}

impl PageStage {
    pub(super) fn new(search_query: ApiSearchInput) -> Self {
        Self { search_query }
    }
}

impl StageDocument for PageStage {
    fn get_stage_documents(&self) -> Vec<Document> {
        vec![
            doc! {
                "$skip": self.search_query.page.unwrap_or(0) * MAX_ITEMS_PER_PAGE
            },
            doc! {
                "$limit": MAX_ITEMS_PER_PAGE
            },
        ]
    }
}
