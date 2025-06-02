use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Count {
    pub total_count: u64,
}
