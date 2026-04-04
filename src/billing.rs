use soroban_sdk::{token, Env};

use crate::events;
use crate::storage;
use crate::types::SubscriptionStatus;

pub fn process_charge(env: &Env, sub_id: u64) -> bool {
    let mut sub = storage::get_sub(env, sub_id);
    let plan = storage::get_plan(env, sub.plan_id);
    let now = env.ledger().timestamp();

    // Handle Paused -> Cancelled transition (one more period while paused)
    if sub.status == SubscriptionStatus::Paused {
        if now >= sub.next_billing_time + plan.period {
            sub.status = SubscriptionStatus::Cancelled;
            sub.cancelled_at = now;
            storage::set_sub(env, &sub);
            storage::remove_plan_sub(env, sub.plan_id, sub.id);
            events::emit_subscription_cancelled(env, sub.id, &sub.subscriber);
        }
        return false;
    }

    // Must be Active for charging
    if sub.status != SubscriptionStatus::Active {
        return false;
    }

    // Not due yet
    if now < sub.next_billing_time {
        return false;
    }

    // Check max periods -> Expired
    if plan.max_periods > 0 && sub.periods_billed >= plan.max_periods {
        sub.status = SubscriptionStatus::Expired;
        storage::set_sub(env, &sub);
        storage::remove_plan_sub(env, sub.plan_id, sub.id);
        events::emit_subscription_expired(env, sub.id, &sub.subscriber);
        return false;
    }

    // Trial period - advance without charging
    if sub.periods_billed < plan.trial_periods {
        sub.periods_billed += 1;
        sub.next_billing_time += plan.period;
        storage::set_sub(env, &sub);
        events::emit_charge_success(env, sub.id, 0, &sub.subscriber);
        return true;
    }

    // Check if grace period expired -> Paused
    if sub.failed_at > 0 && now >= sub.failed_at + plan.grace_period {
        sub.status = SubscriptionStatus::Paused;
        storage::set_sub(env, &sub);
        events::emit_subscription_paused(env, sub.id, &sub.subscriber);
        return false;
    }

    // Pre-check balance and allowance (critical: avoids revert on failure)
    let token_client = token::TokenClient::new(env, &plan.token);
    let contract_addr = env.current_contract_address();
    let balance = token_client.balance(&sub.subscriber);
    let allowance = token_client.allowance(&sub.subscriber, &contract_addr);

    if balance < plan.amount || allowance < plan.amount {
        if sub.failed_at == 0 {
            sub.failed_at = now;
        }
        storage::set_sub(env, &sub);
        events::emit_charge_failed(env, sub.id, &sub.subscriber);
        return false;
    }

    // Perform the actual charge via transfer_from
    token_client.transfer_from(&contract_addr, &sub.subscriber, &plan.merchant, &plan.amount);

    sub.periods_billed += 1;
    sub.next_billing_time += plan.period;
    sub.failed_at = 0;
    storage::set_sub(env, &sub);

    events::emit_charge_success(env, sub.id, plan.amount, &sub.subscriber);
    true
}
