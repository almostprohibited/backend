use std::sync::Arc;

use mongodb_connector::connector::MongoDBConnector;
use tokio::spawn;

use crate::ammo::AmmoMatcher;

pub(crate) mod ammo;

#[tokio::main]
async fn main() {
    let connector = MongoDBConnector::new().await;
    let all_ammo = connector.get_all_live_ammo().await;

    eprintln!("Returned {} with count from DB", all_ammo.len());

    let ammo_matcher = Arc::new(AmmoMatcher::new(all_ammo.clone()));

    let mut handles = vec![];

    for ammo in all_ammo {
        if ammo.metadata.is_none() {
            let matcher = ammo_matcher.clone();

            let handle = spawn(async move { matcher.score(&ammo).await });
            handles.push(handle);
        }
    }

    for handle in handles {
        println!("{}", handle.await.unwrap());
    }
}
