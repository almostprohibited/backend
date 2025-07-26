use std::collections::HashMap;

use common::result::enums::RetailerName;
use retailers::{
    errors::RetailerError,
    retailers::{
        al_flahertys::AlFlahertys, bullseye_north::BullseyeNorth,
        calgary_shooting_centre::CalgaryShootingCentre, canadas_gun_store::CanadasGunStore,
        dante_sports::DanteSports, dominion_outdoors::DominionOutdoors,
        firearms_outlet_canada::FirearmsOutletCanada, g4c_gun_store::G4CGunStore,
        italian_sporting_goods::ItalianSportingGoods, lever_arms::LeverArms,
        rangeview_sports::RangeviewSports, rdsc::Rdsc, reliable_gun::ReliableGun,
        select_shooting_supplies::SelectShootingSupplies, tenda::Tenda,
        the_ammo_source::TheAmmoSource, tillsonburg_gun_shop::Tillsonburg,
        true_north_arms::TrueNorthArms,
    },
    traits::Retailer,
};

type Factory = fn() -> Result<Box<dyn Retailer + Send + Sync>, RetailerError>;

#[rustfmt::skip]
fn retailers() -> HashMap<RetailerName, Factory> {
    // using ::from([]) might work, but I don't know how
    // to get the Rust analyzer to accept a closure inside of a tuple
    let mut retailers: HashMap<RetailerName, Factory> = HashMap::new();

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

    retailers
}

pub(crate) fn get_retailers(
    retailer_filter: Vec<RetailerName>,
) -> Vec<Box<dyn Retailer + Send + Sync>> {
    let mut boxed_retailers: Vec<Box<dyn Retailer + Send + Sync>> = Vec::new();

    let retailers = retailers();
    let mut filted_retailers: Vec<Factory> = Vec::new();

    for retailer in retailer_filter {
        let retailer_factory = retailers
            .get(&retailer)
            .expect(&format!("Expected {retailer:?} to be in mapping"));

        filted_retailers.push(*retailer_factory);
    }

    if filted_retailers.len() == 0 {
        filted_retailers.extend(retailers.into_values());
    }

    for retailer in filted_retailers {
        if let Ok(unwrapped_retailer) = retailer() {
            boxed_retailers.push(unwrapped_retailer);
        } else {
            // discord_webhook
            //     .lock()
            //     .await
            //     .send_error(RetailerName::Tenda, err)
            //     .await
        }
    }

    boxed_retailers
}
