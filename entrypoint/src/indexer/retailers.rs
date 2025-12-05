use crate::clients::{
    base::Client, graphql_client::GqlClient, pagination_client::PaginationClient,
};
use common::result::enums::RetailerName;
use discord::get_indexer_webhook;
use retailers::{
    gql::ProphetRiver,
    html::{
        AlFlahertys, AlSimmons, BartonsBigCountry, BullseyeNorth, CalgaryShootingCentre,
        CanadasGunStore, ClintonSportingGoods, DanteSports, DominionOutdoors, FirearmsOutletCanada,
        G4CGunStore, GreatNorthGun, InterSurplus, InternationalShootingSupplies,
        ItalianSportingGoods, LeverArms, MagDump, Marstar, RangeviewSports, Rdsc, ReliableGun,
        SJHardware, SelectShootingSupplies, SoleyOutdoors, Tenda, TheAmmoSource, Tillsonburg,
        TrueNorthArms, VictoryRidgeSports,
    },
    structures::{GqlRetailerSuper, HtmlRetailerSuper},
};
use std::{collections::HashMap, sync::Arc};
use tokio::{sync::Mutex, task::JoinHandle};

type HtmlRetailerSuperFactory = fn() -> Box<dyn HtmlRetailerSuper>;
type GqlRetailerSuperFactory = fn() -> Box<dyn GqlRetailerSuper>;

#[rustfmt::skip]
fn html_retailers() -> HashMap<RetailerName, HtmlRetailerSuperFactory> {
    // using ::from([]) might work, but I don't know how
    // to get the Rust analyzer to accept a closure inside of a tuple
    let mut retailers: HashMap<RetailerName, HtmlRetailerSuperFactory> = HashMap::new();

    retailers.insert(RetailerName::AlFlahertys, || Box::new(AlFlahertys::new()));
    retailers.insert(RetailerName::BullseyeNorth, || Box::new(BullseyeNorth::new()));
    retailers.insert(RetailerName::CalgaryShootingCentre, || Box::new(CalgaryShootingCentre::new()));
    retailers.insert(RetailerName::ReliableGun, || Box::new(ReliableGun::new()));
    retailers.insert(RetailerName::LeverArms, || Box::new(LeverArms::new()));
    retailers.insert(RetailerName::FirearmsOutletCanada, || Box::new(FirearmsOutletCanada::new()));
    retailers.insert(RetailerName::CanadasGunStore, || Box::new(CanadasGunStore::new()));
    retailers.insert(RetailerName::ItalianSportingGoods, || Box::new(ItalianSportingGoods::new()));
    retailers.insert(RetailerName::TheAmmoSource, || Box::new(TheAmmoSource::new()));
    retailers.insert(RetailerName::Rdsc, || Box::new(Rdsc::new()));
    retailers.insert(RetailerName::G4CGunStore, || Box::new(G4CGunStore::new()));
    retailers.insert(RetailerName::Tillsonburg, || Box::new(Tillsonburg::new()));
    retailers.insert(RetailerName::DanteSports, || Box::new(DanteSports::new()));
    retailers.insert(RetailerName::SelectShootingSupplies, || Box::new(SelectShootingSupplies::new()));
    retailers.insert(RetailerName::RangeviewSports, || Box::new(RangeviewSports::new()));
    retailers.insert(RetailerName::TrueNorthArms, || Box::new(TrueNorthArms::new()));
    retailers.insert(RetailerName::DominionOutdoors, || Box::new(DominionOutdoors::new()));
    retailers.insert(RetailerName::Tenda, || Box::new(Tenda::new()));
    retailers.insert(RetailerName::InternationalShootingSupplies, || Box::new(InternationalShootingSupplies::new()));
    retailers.insert(RetailerName::InterSurplus, || Box::new(InterSurplus::new()));
    retailers.insert(RetailerName::GreatNorthGun, || Box::new(GreatNorthGun::new()));
    retailers.insert(RetailerName::ClintonSportingGoods, || Box::new(ClintonSportingGoods::new()));
    retailers.insert(RetailerName::AlSimmons, || Box::new(AlSimmons::new()));
    retailers.insert(RetailerName::SJHardware, || Box::new(SJHardware::new()));
    retailers.insert(RetailerName::VictoryRidgeSports, || Box::new(VictoryRidgeSports::new()));
    retailers.insert(RetailerName::Marstar, || Box::new(Marstar::new()));
    retailers.insert(RetailerName::MagDump, || Box::new(MagDump::new()));
    retailers.insert(RetailerName::SoleyOutdoors, || Box::new(SoleyOutdoors::new()));
    retailers.insert(RetailerName::BartonsBigCountry, || Box::new(BartonsBigCountry::new()));

    retailers
}

#[rustfmt::skip]
fn gql_retailers() -> HashMap<RetailerName, GqlRetailerSuperFactory> {
    // using ::from([]) might work, but I don't know how
    // to get the Rust analyzer to accept a closure inside of a tuple
    let mut retailers: HashMap<RetailerName, GqlRetailerSuperFactory> = HashMap::new();

    retailers.insert(RetailerName::ProphetRiver, || Box::new(ProphetRiver::new()));

    retailers
}

fn filter_retailers<T: ?Sized>(
    retailer_filter: &[RetailerName],
    excluded_retailer_filter: &[RetailerName],
    retailers: HashMap<RetailerName, fn() -> Box<T>>,
) -> Vec<fn() -> Box<T>> {
    let mut filted_retailers: Vec<fn() -> Box<T>> = Vec::new();

    let included_retailers: Vec<RetailerName> = match retailer_filter.len() {
        0 => retailers.keys().copied().collect(),
        _ => retailer_filter.to_owned(),
    };

    let search_space: Vec<&RetailerName> = included_retailers
        .iter()
        .filter(|retailer| !excluded_retailer_filter.contains(retailer))
        .collect();

    for retailer in search_space {
        if let Some(retailer_factory) = retailers.get(retailer) {
            filted_retailers.push(*retailer_factory);
        }
    }

    filted_retailers
}

// Not sure if this should live inside the Client trait file
// since it's only used here
impl std::fmt::Debug for dyn Client + Send {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Client")
    }
}

// This method contains some repeat code that can probably be
// reduced if I had added an invariant to the constructors
// of both HTML and GQL clients, and moved the client logic to
// filter_retailers(), but that doesn't look nice
pub(crate) async fn get_retailers(
    retailer_filter: Vec<RetailerName>,
    excluded_retailer_filter: Vec<RetailerName>,
) -> Vec<Box<dyn Client + Send>> {
    let boxed_clients: Arc<Mutex<Vec<Box<dyn Client + Send>>>> = Arc::new(Mutex::new(Vec::new()));

    let html_retailers: Vec<HtmlRetailerSuperFactory> = filter_retailers::<dyn HtmlRetailerSuper>(
        &retailer_filter,
        &excluded_retailer_filter,
        html_retailers(),
    );

    let gql_retailers: Vec<GqlRetailerSuperFactory> = filter_retailers::<dyn GqlRetailerSuper>(
        &retailer_filter,
        &excluded_retailer_filter,
        gql_retailers(),
    );

    let mut handles: Vec<JoinHandle<()>> = vec![];

    for retailer in html_retailers {
        let cloned_clients = boxed_clients.clone();

        handles.push(tokio::spawn(async move {
            let mut boxed_retailer = retailer();

            let mut indexer_webhook = get_indexer_webhook().await;
            indexer_webhook.register_retailer(boxed_retailer.get_retailer_name());

            if let Err(error) = boxed_retailer.init().await {
                indexer_webhook
                    .record_retailer_failure(boxed_retailer.get_retailer_name(), error.to_string());
            } else {
                cloned_clients
                    .lock()
                    .await
                    .push(Box::new(PaginationClient::new(boxed_retailer)));
            }
        }));
    }

    for retailer in gql_retailers {
        let cloned_clients = boxed_clients.clone();

        handles.push(tokio::spawn(async move {
            let mut boxed_retailer = retailer();

            let mut indexer_webhook = get_indexer_webhook().await;
            indexer_webhook.register_retailer(boxed_retailer.get_retailer_name());

            if let Err(error) = boxed_retailer.init().await {
                indexer_webhook
                    .record_retailer_failure(boxed_retailer.get_retailer_name(), error.to_string());
            } else {
                cloned_clients
                    .lock()
                    .await
                    .push(Box::new(GqlClient::new(boxed_retailer)));
            }
        }));
    }

    for handle in handles {
        let _ = handle.await;
    }

    get_indexer_webhook().await.update_main_message().await;

    let strong_ref =
        Arc::try_unwrap(boxed_clients).expect("All threads to have dropped their refs");

    strong_ref.into_inner()
}
