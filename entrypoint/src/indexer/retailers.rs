use std::collections::HashMap;

use common::result::enums::RetailerName;
use retailers::{
    errors::RetailerError,
    retailers::{
        gql::prophet_river::prophet_river::ProphetRiver,
        html::{
            al_flahertys::AlFlahertys, bullseye_north::BullseyeNorth,
            calgary_shooting_centre::CalgaryShootingCentre, canadas_gun_store::CanadasGunStore,
            dante_sports::DanteSports, dominion_outdoors::DominionOutdoors,
            firearms_outlet_canada::FirearmsOutletCanada, g4c_gun_store::G4CGunStore,
            great_north_gun::GreatNorthGun,
            international_shooting_supplies::InternationalShootingSupplies,
            intersurplus::InterSurplus, italian_sporting_goods::ItalianSportingGoods,
            lever_arms::LeverArms, rangeview_sports::RangeviewSports, rdsc::Rdsc,
            reliable_gun::ReliableGun, select_shooting_supplies::SelectShootingSupplies,
            tenda::Tenda, the_ammo_source::TheAmmoSource, tillsonburg_gun_shop::Tillsonburg,
            true_north_arms::TrueNorthArms,
        },
    },
    structures::{GqlRetailerSuper, HtmlRetailerSuper},
};

use crate::clients::{
    base::Client, graphql_client::GqlClient, pagination_client::PaginationClient,
};

type HtmlRetailerSuperFactory = fn() -> Result<Box<dyn HtmlRetailerSuper>, RetailerError>;
type GqlRetailerSuperFactory = fn() -> Result<Box<dyn GqlRetailerSuper>, RetailerError>;

#[rustfmt::skip]
fn html_retailers() -> HashMap<RetailerName, fn() -> Result<Box<dyn HtmlRetailerSuper>, RetailerError>> {
    // using ::from([]) might work, but I don't know how
    // to get the Rust analyzer to accept a closure inside of a tuple
    let mut retailers: HashMap<RetailerName, HtmlRetailerSuperFactory> = HashMap::new();

    retailers.insert(RetailerName::AlFlahertys, || Ok(Box::new(AlFlahertys::new())));
    retailers.insert(RetailerName::BullseyeNorth, || Ok(Box::new(BullseyeNorth::new())));
    retailers.insert(RetailerName::CalgaryShootingCentre, || Ok(Box::new(CalgaryShootingCentre::new())));
    retailers.insert(RetailerName::ReliableGun, || Ok(Box::new(ReliableGun::new())));
    retailers.insert(RetailerName::LeverArms, || Ok(Box::new(LeverArms::new())));
    retailers.insert(RetailerName::FirearmsOutletCanada, || Ok(Box::new(FirearmsOutletCanada::new())));
    retailers.insert(RetailerName::CanadasGunStore, || Ok(Box::new(CanadasGunStore::new())));
    retailers.insert(RetailerName::ItalianSportingGoods, || Ok(Box::new(ItalianSportingGoods::new())));
    retailers.insert(RetailerName::TheAmmoSource, || Ok(Box::new(TheAmmoSource::new())));
    retailers.insert(RetailerName::Rdsc, || Ok(Box::new(Rdsc::new())));
    retailers.insert(RetailerName::G4CGunStore, || Ok(Box::new(G4CGunStore::new())));
    retailers.insert(RetailerName::Tillsonburg, || Ok(Box::new(Tillsonburg::new())));
    retailers.insert(RetailerName::DanteSports, || Ok(Box::new(DanteSports::new())));
    retailers.insert(RetailerName::SelectShootingSupplies, || Ok(Box::new(SelectShootingSupplies::new())));
    retailers.insert(RetailerName::RangeviewSports, || Ok(Box::new(RangeviewSports::new())));
    retailers.insert(RetailerName::TrueNorthArms, || Ok(Box::new(TrueNorthArms::new())));
    retailers.insert(RetailerName::DominionOutdoors, || Ok(Box::new(DominionOutdoors::new())));
    retailers.insert(RetailerName::Tenda, || Ok(Box::new(Tenda::new()?)));
    retailers.insert(RetailerName::InternationalShootingSupplies, || Ok(Box::new(InternationalShootingSupplies::new())));
    retailers.insert(RetailerName::InterSurplus, || Ok(Box::new(InterSurplus::new())));
    retailers.insert(RetailerName::GreatNorthGun, || Ok(Box::new(GreatNorthGun::new())));

    retailers
}

#[rustfmt::skip]
fn gql_retailers() -> HashMap<RetailerName, fn() -> Result<Box<dyn GqlRetailerSuper>, RetailerError>> {
    // using ::from([]) might work, but I don't know how
    // to get the Rust analyzer to accept a closure inside of a tuple
    let mut retailers: HashMap<RetailerName, GqlRetailerSuperFactory> = HashMap::new();

    retailers.insert(RetailerName::ProphetRiver, || Ok(Box::new(ProphetRiver::new()?)));

    retailers
}

fn filter_retailers<T: ?Sized>(
    retailer_filter: &Vec<RetailerName>,
    excluded_retailer_filter: &Vec<RetailerName>,
    retailers: HashMap<RetailerName, fn() -> Result<Box<T>, RetailerError>>,
) -> Vec<fn() -> Result<Box<T>, RetailerError>> {
    let mut filted_retailers: Vec<fn() -> Result<Box<T>, RetailerError>> = Vec::new();

    let included_retailers: Vec<RetailerName> = match retailer_filter.len() {
        0 => retailers.keys().copied().collect(),
        _ => retailer_filter.clone(),
    };

    let search_space: Vec<&RetailerName> = included_retailers
        .iter()
        .filter(|retailer| !excluded_retailer_filter.contains(&retailer))
        .collect();

    for retailer in search_space {
        if let Some(retailer_factory) = retailers.get(&retailer) {
            filted_retailers.push(*retailer_factory);
        }
    }

    filted_retailers
}

// This method contains some repeat code that can probably be
// reduced if I had added an invariant to the constructors
// of both HTML and GQL clients, and moved the client logic to
// filter_retailers(), but that doesn't look nice
pub(crate) fn get_retailers(
    retailer_filter: Vec<RetailerName>,
    excluded_retailer_filter: Vec<RetailerName>,
) -> Vec<Box<dyn Client + Send + Sync>> {
    let mut boxed_clients: Vec<Box<dyn Client + Send + Sync>> = Vec::new();

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

    for retailer in html_retailers {
        if let Ok(unwrapped_retailer) = retailer() {
            boxed_clients.push(Box::new(PaginationClient::new(unwrapped_retailer)));
        } else {
            // discord_webhook
            //     .lock()
            //     .await
            //     .send_error(RetailerName::Tenda, err)
            //     .await
        }
    }

    for retailer in gql_retailers {
        if let Ok(unwrapped_retailer) = retailer() {
            boxed_clients.push(Box::new(GqlClient::new(unwrapped_retailer)));
        } else {
            // discord_webhook
            //     .lock()
            //     .await
            //     .send_error(RetailerName::Tenda, err)
            //     .await
        }
    }

    boxed_clients
}
