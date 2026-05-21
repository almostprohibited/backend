use std::collections::HashMap;

use bm25::{Embedder, EmbedderBuilder, Language, Scorer};
use common::result::base::CrawlResult;
use mongodb_connector::connector::MongoDBConnector;
use serde_json::{Value, json};

const MAX_RESULTS: usize = 5;

fn get_document_name(result: &CrawlResult) -> String {
    format!("{} - {}", result.name, result.price.regular_price)
}

#[tokio::main]
async fn main() {
    let connector = MongoDBConnector::new().await;

    eprintln!("Fetching ammo");
    let mut ammo_count: Vec<CrawlResult> = Default::default();
    let mut ammo_no_count: Vec<CrawlResult> = Default::default();

    for ammo in connector.get_all_live_ammo().await {
        if ammo.metadata.is_some() {
            ammo_count.push(ammo);
        } else {
            ammo_no_count.push(ammo);
        }
    }

    eprintln!(
        "Returned {} with count, {} without count from DB",
        ammo_count.len(),
        ammo_no_count.len()
    );

    eprintln!("Converting results into mappings");
    let mut parsed_mapping: HashMap<CrawlResult, String> = Default::default();

    for result in ammo_count {
        // create doc first to avoid a full result clone
        let document = get_document_name(&result);

        parsed_mapping.insert(result, document);
    }

    eprintln!("Generate embeddings");
    let corpus: Vec<&str> = parsed_mapping
        .values()
        .map(|string| string.as_ref())
        .collect();

    let mut scorer = Scorer::<CrawlResult>::new();
    let embedder: Embedder =
        EmbedderBuilder::with_fit_to_corpus(Language::English, &corpus).build();

    for (result, document) in parsed_mapping {
        let embedding = embedder.embed(&document);
        scorer.upsert(&result, embedding);
    }

    eprintln!("Processing counts");
    for ammo in ammo_no_count {
        let query_embedding = embedder.embed(&get_document_name(&ammo));

        let mut matches = scorer.matches(&query_embedding);
        matches.truncate(MAX_RESULTS);

        let json_matches: Vec<Value> = matches
            .iter()
            .map(|matched| {
                json!({
                    "score": matched.score,
                    "document": matched.id
                })
            })
            .collect();

        println!(
            "{}",
            json!({
                "document": ammo,
                "matches": json_matches,
            })
        );
    }
}
