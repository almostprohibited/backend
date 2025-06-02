use mongodb::bson::{Document, doc};

use super::traits::StageDocument;

const MAX_ITEMS_PER_PAGE: u32 = 32;

pub(crate) struct PageStage {
    page: Option<u32>,
}

impl PageStage {
    pub(crate) fn new(page: Option<u32>) -> Self {
        Self { page }
    }
}

impl StageDocument for PageStage {
    fn get_stage_documents(&self) -> Vec<Document> {
        let mut documents: Vec<Document> = Vec::new();

        if let Some(page) = self.page {
            documents.push(doc! {
                "$skip": page * MAX_ITEMS_PER_PAGE
            });
        }

        documents.push(doc! {
            "$limit": MAX_ITEMS_PER_PAGE
        });

        documents
    }
}
