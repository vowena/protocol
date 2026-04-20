use soroban_sdk::{token, Address, Env};

use crate::errors::VowenaError;
use crate::events;
use crate::storage;
use crate::types::{Subscription, SubscriptionStatus};

pub fn process_request_migration(
    env: &Env,
    merchant: &Address,
    old_plan_id: u64,
    new_plan_id: u64,
) -> Result<(), VowenaError> {
    let old_plan = storage::get_plan(env, old_plan_id);
    let new_plan = storage::get_plan(env, new_plan_id);

    if old_plan.merchant != *merchant || new_plan.merchant != *merchant {
        return Err(VowenaError::MerchantMismatch);
    }
    if !new_plan.active {
        return Err(VowenaError::PlanInactive);
    }

    // Set migration_target on all active subs of old plan
    let sub_ids = storage::get_plan_subs(env, old_plan_id);
    for sub_id in sub_ids.iter() {
        let mut sub = storage::get_sub(env, sub_id);
        if sub.status == SubscriptionStatus::Active {
            sub.migration_target = new_plan_id;
            storage::set_sub(env, &sub);
        }
    }

    events::emit_migration_requested(env, old_plan_id, new_plan_id);
    Ok(())
}

pub fn process_accept_migration(
    env: &Env,
    subscriber: &Address,
    sub_id: u64,
    expiration_ledger: u32,
    allowance_periods: u32,
) -> Result<u64, VowenaError> {
    let old_sub = storage::get_sub(env, sub_id);

    if old_sub.subscriber != *subscriber {
        return Err(VowenaError::Unauthorized);
    }
    if old_sub.migration_target == 0 {
        return Err(VowenaError::NoMigrationPending);
    }

    let new_plan = storage::get_plan(env, old_sub.migration_target);

    // Cancel old subscription
    let mut cancelled_sub = old_sub.clone();
    cancelled_sub.status = SubscriptionStatus::Cancelled;
    cancelled_sub.cancelled_at = env.ledger().timestamp();
    cancelled_sub.migration_target = 0;
    storage::set_sub(env, &cancelled_sub);
    storage::remove_plan_sub(env, cancelled_sub.plan_id, cancelled_sub.id);

    // Create new subscription
    let new_sub_id = storage::get_next_sub_id(env);
    storage::set_next_sub_id(env, new_sub_id + 1);

    let now = env.ledger().timestamp();
    let new_sub = Subscription {
        id: new_sub_id,
        plan_id: new_plan.id,
        subscriber: subscriber.clone(),
        status: SubscriptionStatus::Active,
        created_at: now,
        periods_billed: 0,
        next_billing_time: now + new_plan.period,
        failed_at: 0,
        migration_target: 0,
        cancelled_at: 0,
    };

    storage::set_sub(env, &new_sub);
    storage::add_subscriber_sub(env, subscriber, new_sub_id);
    storage::add_plan_sub(env, new_plan.id, new_sub_id);

    // Set new token allowance using caller-provided deterministic params.
    let effective_periods: u32 = if new_plan.max_periods > 0 {
        allowance_periods.min(new_plan.max_periods)
    } else {
        allowance_periods.min(120)
    };
    let allowance = new_plan.price_ceiling * (effective_periods as i128);

    let contract_addr = env.current_contract_address();
    let token_client = token::TokenClient::new(env, &new_plan.token);
    token_client.approve(subscriber, &contract_addr, &allowance, &expiration_ledger);

    events::emit_migration_accepted(env, sub_id, new_sub_id, subscriber);
    Ok(new_sub_id)
}

pub fn process_reject_migration(
    env: &Env,
    subscriber: &Address,
    sub_id: u64,
) -> Result<(), VowenaError> {
    let mut sub = storage::get_sub(env, sub_id);

    if sub.subscriber != *subscriber {
        return Err(VowenaError::Unauthorized);
    }
    if sub.migration_target == 0 {
        return Err(VowenaError::NoMigrationPending);
    }

    sub.migration_target = 0;
    storage::set_sub(env, &sub);

    events::emit_migration_rejected(env, sub_id, subscriber);
    Ok(())
}
