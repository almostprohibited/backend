use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum Category {
    Firearm,
    Ammunition,
    Other,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum RetailerName {
    ReliableGun,
    LeverArms,
    ItalianSportingGoods,
    InternationalShootingSupplies,
    AlFlahertys,
    BullseyeNorth,
    CanadasGunShop,
    CanadasGunStore,
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
