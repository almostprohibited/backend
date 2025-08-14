use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Category {
    Firearm,
    Ammunition,
    Other,
    #[default]
    #[serde(rename = "all")]
    _All,
}

impl Display for Category {
    fn fmt(&self, format: &mut Formatter) -> Result {
        write!(format, "{}", format!("{:?}", self).to_lowercase())
    }
}

#[derive(
    Clone, Copy, Debug, Deserialize, Serialize, Hash, Eq, PartialEq, ValueEnum, Ord, PartialOrd,
)]
pub enum RetailerName {
    ReliableGun,
    LeverArms,
    ItalianSportingGoods,
    AlFlahertys,
    BullseyeNorth,
    CalgaryShootingCentre,
    CanadasGunStore,
    FirearmsOutletCanada,
    TheAmmoSource,
    Tenda,
    Rdsc,
    G4CGunStore,
    Tillsonburg,
    DanteSports,
    SelectShootingSupplies,
    RangeviewSports,
    TrueNorthArms,
    DominionOutdoors,
    ProphetRiver,
}

impl RetailerName {
    pub fn to_string(&self) -> String {
        format!("{self:?}")
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub enum ActionType {
    SemiAuto,
    LeverAction,
    BreakAction,
    BoltAction,
    OverUnder,
    PumpAction,
    SideBySide,
    SingleShot,
    Revolver,
    StraightPull,
    MuzzleLoader,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum AmmunitionType {
    CenterFire,
    Rimfire,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum FirearmClass {
    NonRestricted,
    Restricted,
    Prohibited,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum FirearmType {
    Rifle,
    Shotgun,
}
