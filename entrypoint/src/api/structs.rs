use mongodb_connector::connector::MongoDBConnector;
use serde::Deserialize;

pub(crate) struct ServerState {
    pub(crate) db: MongoDBConnector,
}

#[derive(Deserialize, Debug)]
pub(crate) struct CloudflareResponse {
    pub(crate) success: bool,
    // cloudflare returns more data in the response
    // I don't care about the extra data
}
