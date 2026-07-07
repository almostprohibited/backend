use std::{collections::HashMap, sync::OnceLock, time::Instant};

use bm25::{Embedder, EmbedderBuilder, Language, Scorer};
use common::result::base::CrawlResult;
use serde_json::{Value, json};

const MAX_RESULTS: usize = 5;

static AMMO_SCORER: OnceLock<Scorer<CrawlResult>> = OnceLock::new();
static AMMO_EMBEDDER: OnceLock<Embedder> = OnceLock::new();

pub(crate) struct AmmoMatcher {
    parsed_mapping: HashMap<CrawlResult, String>,
}

fn get_document_name(result: &CrawlResult) -> String {
    format!("{} -- {}", result.price.regular_price, result.name)
}

impl AmmoMatcher {
    fn get_embedder(parsed_mapping: HashMap<CrawlResult, String>) -> &'static Embedder {
        AMMO_EMBEDDER.get_or_init(|| {
            eprintln!("Embedding docs");

            let corpus: Vec<&str> = parsed_mapping
                .values()
                .map(|string| string.as_ref())
                .collect();

            let embedder: Embedder =
                EmbedderBuilder::with_fit_to_corpus(Language::English, &corpus).build();

            embedder
        })
    }

    fn get_scorer(parsed_mapping: HashMap<CrawlResult, String>) -> &'static Scorer<CrawlResult> {
        AMMO_SCORER.get_or_init(|| {
            eprintln!("Generating scorer");

            let mut scorer = Scorer::<CrawlResult>::new();

            // TODO: don't know if this will deadlock or not
            // pretty sure it won't since everything is synchronous
            // at this point
            let embedder = Self::get_embedder(parsed_mapping.clone());

            for (result, document) in parsed_mapping {
                let embedding = embedder.embed(&document);
                scorer.upsert(&result, embedding);
            }

            scorer
        })
    }

    pub(crate) fn new(all_ammo: Vec<CrawlResult>) -> Self {
        let mut ammo_count: Vec<CrawlResult> = Default::default();

        for ammo in all_ammo {
            if ammo.metadata.is_some() {
                ammo_count.push(ammo);
            }
        }

        let mut parsed_mapping: HashMap<CrawlResult, String> = Default::default();

        for result in ammo_count {
            // create doc first to avoid a full result clone
            let document = get_document_name(&result);

            parsed_mapping.insert(result, document);
        }

        Self { parsed_mapping }
    }

    pub(crate) async fn score(&self, crawl_result: &CrawlResult) -> Value {
        let start = Instant::now();

        let embedder = Self::get_embedder(self.parsed_mapping.clone());
        let scorer = Self::get_scorer(self.parsed_mapping.clone());

        let query_embedding = embedder.embed(&get_document_name(crawl_result));

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

        eprintln!(
            "Scoring document took {} seconds",
            start.elapsed().as_secs_f32()
        );

        json!({
            "document": crawl_result,
            "matches": json_matches,
        })
    }
}
