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
        // TODO: wtf
        let final_price = doc! {
            "$addFields": {
                "product_price": {
                    "$ifNull": ["$price.sale_price", "$price.regular_price"]
                },
                "round_count": {
                    "$ifNull": ["$metadata.Ammunition.round_count", 0]
                },
                "product_price_by_round": {
                    "$cond": {
                        "if": {
                            "$gt": ["$round_count", 0]
                        },
                        "then": {
                            "$divide": ["$product_price", "$round_count"]
                        },
                        "else": null
                    },
                },
                "final_price": {
                    "$ifNull": ["$product_price_by_round", "$price.sale_price", "$price.regular_price"]
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
                // doc! {
                //     "$sort": {
                //         "score": -1,
                //         "_id": 1
                //     }
                // },
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
