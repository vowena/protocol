use soroban_sdk::{contracttype, Address, Env, Vec};

use crate::types::{Plan, Project, Subscription};

// TTL constants (in ledgers, ~5s per ledger)
pub const PERSISTENT_TTL_THRESHOLD: u32 = 518_400; // ~30 days
pub const PERSISTENT_TTL_EXTEND: u32 = 2_073_600; // ~120 days
pub const INSTANCE_TTL_THRESHOLD: u32 = 518_400; // ~30 days
pub const INSTANCE_TTL_EXTEND: u32 = 1_555_200; // ~90 days
// Must stay below the Stellar Asset Contract's live_until max (currently
// 3_110_400 ledgers / ~180 days on testnet+mainnet). If this exceeds the SAC
// cap, token.approve() traps with Error(Contract, #9) during subscribe.
pub const MAX_APPROVAL_LEDGERS: u32 = 3_000_000; // ~173 days

// Minimum approval lifetime, in ledgers. Must exceed Soroban's persistent
// minimum TTL so token.approve() doesn't trap on very short-period plans.
pub const MIN_APPROVAL_LEDGERS: u32 = 17_280; // ~24 hours

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    NextPlanId,
    NextSubId,
    NextProjectId,
    Plan(u64),
    Sub(u64),
    Project(u64),
    MerchantPlans(Address),
    MerchantProjects(Address),
    SubscriberSubs(Address),
    PlanSubs(u64),
}

// --- Instance storage ---

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
}

pub fn has_admin(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Admin)
}

pub fn get_next_plan_id(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get(&DataKey::NextPlanId)
        .unwrap_or(0)
}

pub fn set_next_plan_id(env: &Env, id: u64) {
    env.storage().instance().set(&DataKey::NextPlanId, &id);
}

pub fn get_next_sub_id(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get(&DataKey::NextSubId)
        .unwrap_or(0)
}

pub fn set_next_sub_id(env: &Env, id: u64) {
    env.storage().instance().set(&DataKey::NextSubId, &id);
}

pub fn get_next_project_id(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get(&DataKey::NextProjectId)
        .unwrap_or(0)
}

pub fn set_next_project_id(env: &Env, id: u64) {
    env.storage().instance().set(&DataKey::NextProjectId, &id);
}

// --- Project storage ---

pub fn get_project(env: &Env, id: u64) -> Project {
    env.storage()
        .persistent()
        .get(&DataKey::Project(id))
        .unwrap()
}

pub fn has_project(env: &Env, id: u64) -> bool {
    env.storage().persistent().has(&DataKey::Project(id))
}

pub fn set_project(env: &Env, project: &Project) {
    env.storage()
        .persistent()
        .set(&DataKey::Project(project.id), project);
    bump_project(env, project.id);
}

pub fn add_merchant_project(env: &Env, merchant: &Address, project_id: u64) {
    let key = DataKey::MerchantProjects(merchant.clone());
    let mut ids: Vec<u64> = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or(Vec::new(env));
    ids.push_back(project_id);
    env.storage().persistent().set(&key, &ids);
    env.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND);
}

pub fn get_merchant_projects(env: &Env, merchant: &Address) -> Vec<u64> {
    let key = DataKey::MerchantProjects(merchant.clone());
    env.storage()
        .persistent()
        .get(&key)
        .unwrap_or(Vec::new(env))
}

pub fn bump_project(env: &Env, project_id: u64) {
    env.storage().persistent().extend_ttl(
        &DataKey::Project(project_id),
        PERSISTENT_TTL_THRESHOLD,
        PERSISTENT_TTL_EXTEND,
    );
}

// --- Plan storage ---

pub fn get_plan(env: &Env, id: u64) -> Plan {
    env.storage().persistent().get(&DataKey::Plan(id)).unwrap()
}

pub fn has_plan(env: &Env, id: u64) -> bool {
    env.storage().persistent().has(&DataKey::Plan(id))
}

pub fn set_plan(env: &Env, plan: &Plan) {
    env.storage()
        .persistent()
        .set(&DataKey::Plan(plan.id), plan);
    bump_plan(env, plan.id);
}

// --- Subscription storage ---

pub fn get_sub(env: &Env, id: u64) -> Subscription {
    env.storage().persistent().get(&DataKey::Sub(id)).unwrap()
}

pub fn has_sub(env: &Env, id: u64) -> bool {
    env.storage().persistent().has(&DataKey::Sub(id))
}

pub fn set_sub(env: &Env, sub: &Subscription) {
    env.storage().persistent().set(&DataKey::Sub(sub.id), sub);
    bump_sub(env, sub.id);
}

// --- Index helpers ---

pub fn add_merchant_plan(env: &Env, merchant: &Address, plan_id: u64) {
    let key = DataKey::MerchantPlans(merchant.clone());
    let mut ids: Vec<u64> = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or(Vec::new(env));
    ids.push_back(plan_id);
    env.storage().persistent().set(&key, &ids);
    env.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND);
}

pub fn get_merchant_plans(env: &Env, merchant: &Address) -> Vec<u64> {
    let key = DataKey::MerchantPlans(merchant.clone());
    env.storage()
        .persistent()
        .get(&key)
        .unwrap_or(Vec::new(env))
}

pub fn add_subscriber_sub(env: &Env, subscriber: &Address, sub_id: u64) {
    let key = DataKey::SubscriberSubs(subscriber.clone());
    let mut ids: Vec<u64> = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or(Vec::new(env));
    ids.push_back(sub_id);
    env.storage().persistent().set(&key, &ids);
    env.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND);
}

pub fn get_subscriber_subs(env: &Env, subscriber: &Address) -> Vec<u64> {
    let key = DataKey::SubscriberSubs(subscriber.clone());
    env.storage()
        .persistent()
        .get(&key)
        .unwrap_or(Vec::new(env))
}

pub fn add_plan_sub(env: &Env, plan_id: u64, sub_id: u64) {
    let key = DataKey::PlanSubs(plan_id);
    let mut ids: Vec<u64> = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or(Vec::new(env));
    ids.push_back(sub_id);
    env.storage().persistent().set(&key, &ids);
    env.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND);
}

pub fn get_plan_subs(env: &Env, plan_id: u64) -> Vec<u64> {
    let key = DataKey::PlanSubs(plan_id);
    env.storage()
        .persistent()
        .get(&key)
        .unwrap_or(Vec::new(env))
}

pub fn remove_plan_sub(env: &Env, plan_id: u64, sub_id: u64) {
    let key = DataKey::PlanSubs(plan_id);
    let ids: Vec<u64> = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or(Vec::new(env));
    let mut new_ids = Vec::new(env);
    for id in ids.iter() {
        if id != sub_id {
            new_ids.push_back(id);
        }
    }
    env.storage().persistent().set(&key, &new_ids);
}

// --- TTL bump helpers ---

pub fn bump_plan(env: &Env, plan_id: u64) {
    env.storage().persistent().extend_ttl(
        &DataKey::Plan(plan_id),
        PERSISTENT_TTL_THRESHOLD,
        PERSISTENT_TTL_EXTEND,
    );
}

pub fn bump_sub(env: &Env, sub_id: u64) {
    env.storage().persistent().extend_ttl(
        &DataKey::Sub(sub_id),
        PERSISTENT_TTL_THRESHOLD,
        PERSISTENT_TTL_EXTEND,
    );
}

pub fn bump_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND);
}
