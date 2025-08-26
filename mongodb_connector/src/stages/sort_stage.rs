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
        let mut sort_docs: Vec<Document> = vec![];

        // TODO: wtf
        let final_price = vec![
            doc! {
                "$addFields": {
                    "product_price": {
                        "$ifNull": ["$price.sale_price", "$price.regular_price"]
                    },
                    "round_count": {
                        "$ifNull": [
                            {
                                "$getField": {
                                    "field": "round_count",
                                    "input": {
                                        "$getField": {
                                            "field": "Ammunition",
                                            "input": "$metadata"
                                        }
                                    }
                                }
                            },
                            0
                        ]
                    },

                }
            },
            doc! {
                "$addFields": {
                    "product_price_by_round": {
                        "$cond": {
                            "if": {
                                "$gt": ["$round_count", 0]
                            },
                            "then": {
                                "$divide": ["$product_price", "$round_count"]
                            },
                            "else": null
                        }
                    }
                }
            },
            doc! {
                "$addFields": {
                    "final_price": {
                        "$ifNull": ["$product_price_by_round", "$product_price"]
                    }
                }
            },
        ];

        match self.sort {
            Sort::Relevant => {
                sort_docs.extend([
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
                            "name": 1,
                        }
                    },
                ]);
            }
            Sort::PriceAsc => {
                sort_docs.extend(final_price);
                sort_docs.push(doc! {
                    "$sort": {
                        "final_price": 1,
                        "product_price": 1,
                        "name": 1,
                    }
                });
            }
            Sort::PriceDesc => {
                sort_docs.extend(final_price);
                sort_docs.push(doc! {
                    "$sort": {
                        "final_price": -1,
                        "product_price": -1,
                        "name": 1,
                    }
                });
            }
        };

        sort_docs
    }
}
