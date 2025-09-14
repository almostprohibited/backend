use common::{result::enums::Category, utils::normalized_relative_days};
use mongodb::bson::{Document, doc};
use tracing::trace;

use super::traits::StageDocument;

pub(super) struct MatchStage {
    query: String,
    category: Category,
    min_price: Option<u32>,
    max_price: Option<u32>,
}

impl MatchStage {
    pub(super) fn new(
        query: String,
        category: Category,
        min_price: Option<u32>,
        max_price: Option<u32>,
    ) -> Self {
        let mut search_terms = query
            .split(" ")
            .map(|term| format!("\"{term}\""))
            .collect::<Vec<String>>();

        search_terms.sort();

        Self {
            query: search_terms.join(" "),
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
}

impl StageDocument for MatchStage {
    fn get_stage_documents(&self) -> Vec<Document> {
        let mut match_filter = doc! {
            "$text": {
                "$search": &self.query
            },
            "query_time": {
                "$gte": normalized_relative_days(2)
            }
        };

        let price_filter = self.get_price_documents();

        trace!("Price filters: {:#?}", price_filter);

        if !price_filter.is_empty() {
            match_filter.insert(
                "$expr",
                doc! {
                    "$and": price_filter
                },
            );
        }

        let all_category = vec![
            Category::Firearm.to_string(),
            Category::Other.to_string(),
            Category::Ammunition.to_string(),
        ];

        if self.category == Category::default() {
            match_filter.insert(
                "category",
                doc! {
                    "$in": all_category
                },
            );
        } else {
            match_filter.insert("category", self.category.to_string());
        }

        [doc! {"$match": match_filter}].into()
    }
}
