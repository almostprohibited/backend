#[derive(Debug, Copy, Clone)]
pub enum ActionType {
    SemiAuto,
    LeverAction,
    BreakAction,
    BoltAction,
    OverUnder,
    PumpAction,
    SideBySide,
    SingleShot,
}

#[derive(Debug, Copy, Clone)]
pub enum AmmunitionType {
    CenterFire,
    Rimfire,
}

#[derive(Debug, Copy, Clone)]
pub enum FirearmClass {
    NonRestricted,
    Restricted,
    Prohibited,
}

#[derive(Debug, Copy, Clone)]
pub enum FirearmType {
    Rifle,
    Shotgun,
}
