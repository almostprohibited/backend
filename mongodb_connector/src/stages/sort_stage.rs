use mongodb::bson::{Document, doc};

use super::traits::{Sort, StageDocument};

pub(crate) struct SortStage {
    sort: Sort,
}

impl SortStage {
    pub(crate) fn new(sort: Sort) -> Self {
        Self { sort }
    }
}

impl StageDocument for SortStage {
    fn get_stage_documents(&self) -> Vec<Document> {
        let final_price = doc! {
            "$addFields": {
                "final_price": {
                    "$ifNull": ["$price.sale_price", "$price.regular_price"]
                }
            }
        };

        if let Sort::Relevant = self.sort {
            let docs = [
                doc! {
                    "$addFields": {
                        "score": {
                            "$meta": "textScore"
                        }
                    }
                },
                doc! {
                    "$sort": {
                        "score": -1,
                        "_id": 1
                    }
                },
            ];

            return docs.into();
        };

        if let Sort::PriceAsc = self.sort {
            let docs = [
                final_price,
                doc! {
                    "$sort": {
                        "final_price": 1
                    }
                },
            ];

            return docs.into();
        };

        if let Sort::PriceDesc = self.sort {
            let docs = [
                final_price,
                doc! {
                    "$sort": {
                        "final_price": -1
                    }
                },
            ];

            return docs.into();
        };

        [].into()
    }
}
