use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum VowenaError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    InvalidAmount = 3,
    InvalidPeriod = 4,
    CeilingBelowAmount = 5,
    PlanNotFound = 6,
    PlanInactive = 7,
    SubNotFound = 8,
    Unauthorized = 9,
    AmountExceedsCeiling = 10,
    MerchantMismatch = 11,
    NoMigrationPending = 12,
    NotPaused = 13,
}
