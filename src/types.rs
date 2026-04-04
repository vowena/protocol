use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum SubscriptionStatus {
    Active,
    Paused,
    Cancelled,
    Expired,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct Plan {
    pub id: u64,
    pub merchant: Address,
    pub token: Address,
    pub amount: i128,
    pub period: u64,
    pub trial_periods: u32,
    pub max_periods: u32,
    pub grace_period: u64,
    pub price_ceiling: i128,
    pub created_at: u64,
    pub active: bool,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct Subscription {
    pub id: u64,
    pub plan_id: u64,
    pub subscriber: Address,
    pub status: SubscriptionStatus,
    pub created_at: u64,
    pub periods_billed: u32,
    pub next_billing_time: u64,
    pub failed_at: u64,
    pub migration_target: u64,
    pub cancelled_at: u64,
}
