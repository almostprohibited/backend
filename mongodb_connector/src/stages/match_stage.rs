use std::time::{SystemTime, UNIX_EPOCH};

use common::result::enums::Category;
use mongodb::bson::{Document, doc};
use tracing::trace;

use super::traits::StageDocument;

pub(crate) struct MatchStage {
    query: String,
    category: Category,
    min_price: Option<u32>,
    max_price: Option<u32>,
}

impl MatchStage {
    pub(crate) fn new(
        query: String,
        category: Category,
        min_price: Option<u32>,
        max_price: Option<u32>,
    ) -> Self {
        let search_terms = query
            .split(" ")
            .map(|term| format!("\"{}\"", term))
            .collect::<Vec<String>>()
            .join(" ");

        Self {
            query: search_terms,
            category,
            min_price,
            max_price,
        }
    }

    fn get_price_documents(&self) -> Vec<Document> {
        let final_price_doc = doc! {
            "$ifNull": ["$price.sale_price", "$price.regular_price"]
        };

        let mut documents: Vec<Document> = Vec::new();

        if let Some(min_price) = self.min_price {
            documents.push(doc! {
                "$gte": [final_price_doc.clone(), min_price]
            });
        }

        if let Some(max_price) = self.max_price {
            documents.push(doc! {
                "$lte": [final_price_doc, max_price]
            });
        }

        documents
    }

    fn relative_time_document(&self) -> Document {
        let two_days_ago_secs: i64 = 2 * 24 * 60 * 60;

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap() // this should not fail since the current time is always > UNIX_EPOCH
            .as_secs() as i64;

        doc! {
            "$gte": current_time - two_days_ago_secs
        }
    }
}

impl StageDocument for MatchStage {
    fn get_stage_documents(&self) -> Vec<Document> {
        let mut match_filter = doc! {
            "$text": {
                "$search": &self.query
            },
            "query_time": self.relative_time_document()
        };

        let price_filter = self.get_price_documents();

        trace!("Price filters: {:#?}", price_filter);

        if price_filter.len() > 0 {
            match_filter.insert(
                "$expr",
                doc! {
                    "$and": price_filter
                },
            );
        }

        if self.category != Category::default() {
            match_filter.insert("category", format!("{}", self.category));
        }

        [doc! {"$match": match_filter}].into()
    }
}
