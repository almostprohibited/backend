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

        let document = match self.sort {
            Sort::Relevant => doc! {
                "$sort": {
                    "score": {
                        "$meta": "textScore"
                    }
                }
            },
            Sort::PriceAsc => doc! {
                "$sort": {
                    "final_price": 1
                }
            },
            Sort::PriceDesc => doc! {
                "$sort": {
                    "final_price": -1
                }
            },
        };

        if let Sort::Relevant = self.sort {
            return [document].into();
        };

        [final_price, document].into()
    }
}
