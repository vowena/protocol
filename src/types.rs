use soroban_sdk::{contracttype, Address, String};

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
pub struct Project {
    pub id: u64,
    pub merchant: Address,
    pub name: String,
    pub description: String,
    pub created_at: u64,
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
    /// Display name set by the merchant at create time (max ~64 chars on chain).
    pub name: String,
    /// Chain-assigned ID of the parent Project this plan belongs to.
    pub project_id: u64,
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
