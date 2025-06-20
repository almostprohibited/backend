use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
pub enum RetailerName {
    ReliableGun,
    LeverArms,
    ItalianSportingGoods,
    InternationalShootingSupplies,
    AlFlahertys,
    BullseyeNorth,
    CanadasGunShop,
    CanadasGunStore,
    _Unused,
}

impl Default for RetailerName {
    fn default() -> Self {
        Self::_Unused
    }
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
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

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
pub enum AmmunitionType {
    CenterFire,
    Rimfire,
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
pub enum FirearmClass {
    NonRestricted,
    Restricted,
    Prohibited,
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
pub enum FirearmType {
    Rifle,
    Shotgun,
}
