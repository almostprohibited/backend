use common::{
    result::enums::Category, search_params::ApiSearchInput, utils::normalized_relative_days,
};
use mongodb::bson::{Document, doc};
use tracing::trace;

use super::traits::StageDocument;

pub(super) struct MatchStage {
    search_query: ApiSearchInput,
}

impl MatchStage {
    pub(super) fn new(search_query: ApiSearchInput) -> Self {
        Self { search_query }
    }

    fn parse_search_terms(&self) -> String {
        let mut terms = self
            .search_query
            .query
            .split(" ")
            .map(|term| format!("\"{term}\""))
            .collect::<Vec<String>>();

        terms.sort();

        terms.join(" ")
    }

    fn get_price_documents(&self) -> Vec<Document> {
        let final_price_doc = doc! {
            "$ifNull": ["$price.sale_price", "$price.regular_price"]
        };

        let mut documents: Vec<Document> = Vec::new();

        if let Some(min_price) = self.search_query.min_price {
            documents.push(doc! {
                "$gte": [final_price_doc.clone(), min_price]
            });
        }

        if let Some(max_price) = self.search_query.max_price {
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
                "$search": &self.parse_search_terms()
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

        if self.search_query.category == Category::default() {
            match_filter.insert(
                "category",
                doc! {
                    "$in": all_category
                },
            );
        } else {
            match_filter.insert("category", self.search_query.category.to_string());
        }

        [doc! {"$match": match_filter}].into()
    }
}
