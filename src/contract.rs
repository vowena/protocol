#![allow(clippy::too_many_arguments)]

use soroban_sdk::{contract, contractimpl, token, Address, Env, String, Vec};

use crate::billing;
use crate::errors::VowenaError;
use crate::events;
use crate::migration;
use crate::storage;
use crate::types::{Plan, Subscription, SubscriptionStatus};

#[contract]
pub struct VowenaContract;

#[contractimpl]
impl VowenaContract {
    /// Initialize the contract with an admin address. Can only be called once.
    pub fn initialize(env: Env, admin: Address) -> Result<(), VowenaError> {
        if storage::has_admin(&env) {
            return Err(VowenaError::AlreadyInitialized);
        }
        storage::set_admin(&env, &admin);
        storage::set_next_plan_id(&env, 1);
        storage::set_next_sub_id(&env, 1);
        storage::bump_instance(&env);
        Ok(())
    }

    /// Create a new billing plan. Returns the plan ID.
    pub fn create_plan(
        env: Env,
        merchant: Address,
        token: Address,
        amount: i128,
        period: u64,
        trial_periods: u32,
        max_periods: u32,
        grace_period: u64,
        price_ceiling: i128,
        name: String,
        project_slot: u32,
    ) -> Result<u64, VowenaError> {
        merchant.require_auth();

        if amount <= 0 {
            return Err(VowenaError::InvalidAmount);
        }
        if period == 0 {
            return Err(VowenaError::InvalidPeriod);
        }
        if price_ceiling < amount {
            return Err(VowenaError::CeilingBelowAmount);
        }

        let plan_id = storage::get_next_plan_id(&env);
        storage::set_next_plan_id(&env, plan_id + 1);

        let plan = Plan {
            id: plan_id,
            merchant: merchant.clone(),
            token,
            amount,
            period,
            trial_periods,
            max_periods,
            grace_period,
            price_ceiling,
            created_at: env.ledger().timestamp(),
            active: true,
            name,
            project_slot,
        };

        storage::set_plan(&env, &plan);
        storage::add_merchant_plan(&env, &merchant, plan_id);
        storage::bump_instance(&env);

        events::emit_plan_created(&env, plan_id, &merchant);
        Ok(plan_id)
    }

    /// Subscribe to a plan. Sets token allowance and creates subscription. Returns sub ID.
    /// Subscribe to a plan. Caller must supply the absolute `expiration_ledger`
    /// for the SAC allowance (typically `current_ledger + MAX_APPROVAL_LEDGERS`)
    /// and `allowance_periods` to compute the total spending cap. These are
    /// parameters rather than contract-computed values so that the nested
    /// `token.approve` call's args match exactly between transaction simulation
    /// and on-chain submission — without this, Soroban's auth tree rejects the
    /// nested invocation.
    pub fn subscribe(
        env: Env,
        subscriber: Address,
        plan_id: u64,
        expiration_ledger: u32,
        allowance_periods: u32,
    ) -> Result<u64, VowenaError> {
        subscriber.require_auth();

        if !storage::has_plan(&env, plan_id) {
            return Err(VowenaError::PlanNotFound);
        }

        let plan = storage::get_plan(&env, plan_id);
        if !plan.active {
            return Err(VowenaError::PlanInactive);
        }

        // Clamp allowance periods to the plan's max (or a safe ceiling for
        // unlimited plans) so the caller can't inflate the allowance cap.
        let effective_periods: u32 = if plan.max_periods > 0 {
            allowance_periods.min(plan.max_periods)
        } else {
            allowance_periods.min(120)
        };
        let allowance = plan.price_ceiling * (effective_periods as i128);

        let contract_addr = env.current_contract_address();
        let token_client = token::TokenClient::new(&env, &plan.token);
        token_client.approve(&subscriber, &contract_addr, &allowance, &expiration_ledger);

        // Create subscription
        let sub_id = storage::get_next_sub_id(&env);
        storage::set_next_sub_id(&env, sub_id + 1);

        let now = env.ledger().timestamp();
        let mut sub = Subscription {
            id: sub_id,
            plan_id,
            subscriber: subscriber.clone(),
            status: SubscriptionStatus::Active,
            created_at: now,
            periods_billed: 0,
            next_billing_time: now + plan.period,
            failed_at: 0,
            migration_target: 0,
            cancelled_at: 0,
        };

        // Stripe-style semantics: charge on signup unless the plan has a trial.
        // The allowance we just approved covers this pull.
        if plan.trial_periods == 0 && plan.amount > 0 {
            token_client.transfer_from(
                &contract_addr,
                &subscriber,
                &plan.merchant,
                &plan.amount,
            );
            sub.periods_billed = 1;
            events::emit_charge_success(&env, sub_id, plan.amount, &subscriber);
        }

        storage::set_sub(&env, &sub);
        storage::add_subscriber_sub(&env, &subscriber, sub_id);
        storage::add_plan_sub(&env, plan_id, sub_id);
        storage::bump_instance(&env);

        events::emit_subscription_created(&env, sub_id, plan_id, &subscriber);
        Ok(sub_id)
    }

    /// Charge a subscription. Permissionless - anyone can call.
    /// Returns true on successful charge, false otherwise.
    pub fn charge(env: Env, sub_id: u64) -> bool {
        if !storage::has_sub(&env, sub_id) {
            return false;
        }
        billing::process_charge(&env, sub_id)
    }

    /// Cancel a subscription. Caller must be subscriber or merchant.
    pub fn cancel(env: Env, caller: Address, sub_id: u64) -> Result<(), VowenaError> {
        caller.require_auth();

        if !storage::has_sub(&env, sub_id) {
            return Err(VowenaError::SubNotFound);
        }

        let mut sub = storage::get_sub(&env, sub_id);
        let plan = storage::get_plan(&env, sub.plan_id);

        if caller != sub.subscriber && caller != plan.merchant {
            return Err(VowenaError::Unauthorized);
        }

        sub.status = SubscriptionStatus::Cancelled;
        sub.cancelled_at = env.ledger().timestamp();
        storage::set_sub(&env, &sub);
        storage::remove_plan_sub(&env, sub.plan_id, sub.id);

        events::emit_subscription_cancelled(&env, sub.id, &sub.subscriber);
        Ok(())
    }

    /// Refund a subscriber. Must be called by the plan's merchant.
    pub fn refund(env: Env, sub_id: u64, amount: i128) -> Result<(), VowenaError> {
        if !storage::has_sub(&env, sub_id) {
            return Err(VowenaError::SubNotFound);
        }

        let sub = storage::get_sub(&env, sub_id);
        let plan = storage::get_plan(&env, sub.plan_id);

        plan.merchant.require_auth();

        let token_client = token::TokenClient::new(&env, &plan.token);
        token_client.transfer(&plan.merchant, &sub.subscriber, &amount);

        events::emit_refund_issued(&env, sub.id, amount, &sub.subscriber);
        Ok(())
    }

    /// Update a plan's billing amount. Must stay within price ceiling.
    pub fn update_plan_amount(env: Env, plan_id: u64, new_amount: i128) -> Result<(), VowenaError> {
        if !storage::has_plan(&env, plan_id) {
            return Err(VowenaError::PlanNotFound);
        }

        let mut plan = storage::get_plan(&env, plan_id);
        plan.merchant.require_auth();

        if new_amount <= 0 {
            return Err(VowenaError::InvalidAmount);
        }
        if new_amount > plan.price_ceiling {
            return Err(VowenaError::AmountExceedsCeiling);
        }

        plan.amount = new_amount;
        storage::set_plan(&env, &plan);

        events::emit_plan_amount_updated(&env, plan_id, new_amount);
        Ok(())
    }

    /// Request migration of all subs from old plan to new plan. Both must belong to caller.
    pub fn request_migration(
        env: Env,
        merchant: Address,
        old_plan_id: u64,
        new_plan_id: u64,
    ) -> Result<(), VowenaError> {
        merchant.require_auth();
        migration::process_request_migration(&env, &merchant, old_plan_id, new_plan_id)
    }

    /// Accept a pending migration. Caller must supply the absolute
    /// `expiration_ledger` and `allowance_periods` for the new allowance
    /// (same deterministic-auth rationale as `subscribe`).
    pub fn accept_migration(
        env: Env,
        subscriber: Address,
        sub_id: u64,
        expiration_ledger: u32,
        allowance_periods: u32,
    ) -> Result<u64, VowenaError> {
        subscriber.require_auth();
        migration::process_accept_migration(
            &env,
            &subscriber,
            sub_id,
            expiration_ledger,
            allowance_periods,
        )
    }

    /// Reject a pending migration. Subscriber stays on current plan.
    pub fn reject_migration(env: Env, subscriber: Address, sub_id: u64) -> Result<(), VowenaError> {
        subscriber.require_auth();
        migration::process_reject_migration(&env, &subscriber, sub_id)
    }

    /// Reactivate a paused subscription. Caller must supply the absolute
    /// `expiration_ledger` and `allowance_periods` for the SAC re-approval
    /// (same rationale as `subscribe` — keeps auth tree deterministic).
    pub fn reactivate(
        env: Env,
        subscriber: Address,
        sub_id: u64,
        expiration_ledger: u32,
        allowance_periods: u32,
    ) -> Result<bool, VowenaError> {
        subscriber.require_auth();

        if !storage::has_sub(&env, sub_id) {
            return Err(VowenaError::SubNotFound);
        }

        let mut sub = storage::get_sub(&env, sub_id);
        if sub.subscriber != subscriber {
            return Err(VowenaError::Unauthorized);
        }
        if sub.status != SubscriptionStatus::Paused {
            return Err(VowenaError::NotPaused);
        }

        let plan = storage::get_plan(&env, sub.plan_id);

        // Clamp caller-provided periods to remaining authorized periods.
        let effective_periods: u32 = if plan.max_periods > 0 {
            allowance_periods.min(plan.max_periods.saturating_sub(sub.periods_billed))
        } else {
            allowance_periods.min(120)
        };
        let allowance = plan.price_ceiling * (effective_periods as i128);

        let contract_addr = env.current_contract_address();
        let token_client = token::TokenClient::new(&env, &plan.token);
        token_client.approve(&subscriber, &contract_addr, &allowance, &expiration_ledger);

        // Set back to Active and attempt charge
        sub.status = SubscriptionStatus::Active;
        sub.failed_at = 0;
        sub.next_billing_time = env.ledger().timestamp();
        storage::set_sub(&env, &sub);
        storage::add_plan_sub(&env, sub.plan_id, sub.id);

        let charged = billing::process_charge(&env, sub_id);

        events::emit_subscription_reactivated(&env, sub_id, &subscriber);
        Ok(charged)
    }

    // --- Read-only functions ---

    pub fn get_plan(env: Env, plan_id: u64) -> Result<Plan, VowenaError> {
        if !storage::has_plan(&env, plan_id) {
            return Err(VowenaError::PlanNotFound);
        }
        Ok(storage::get_plan(&env, plan_id))
    }

    pub fn get_subscription(env: Env, sub_id: u64) -> Result<Subscription, VowenaError> {
        if !storage::has_sub(&env, sub_id) {
            return Err(VowenaError::SubNotFound);
        }
        Ok(storage::get_sub(&env, sub_id))
    }

    pub fn get_merchant_plans(env: Env, merchant: Address) -> Vec<u64> {
        storage::get_merchant_plans(&env, &merchant)
    }

    pub fn get_subscriber_subscriptions(env: Env, subscriber: Address) -> Vec<u64> {
        storage::get_subscriber_subs(&env, &subscriber)
    }

    pub fn get_plan_subscribers(env: Env, plan_id: u64) -> Vec<u64> {
        storage::get_plan_subs(&env, plan_id)
    }

    pub fn extend_ttl(env: Env, plan_id: u64, sub_id: u64) {
        if storage::has_plan(&env, plan_id) {
            storage::bump_plan(&env, plan_id);
        }
        if storage::has_sub(&env, sub_id) {
            storage::bump_sub(&env, sub_id);
        }
        storage::bump_instance(&env);
    }
}
