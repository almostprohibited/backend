use mongodb_connector::connector::MongoDBConnector;

pub(crate) struct ServerState {
    pub(crate) db: MongoDBConnector,
}
