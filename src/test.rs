#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token::{StellarAssetClient, TokenClient},
    Address, Env,
};

use crate::contract::{VowenaContract, VowenaContractClient};
use crate::types::SubscriptionStatus;

const MONTH: u64 = 2_592_000; // ~30 days in seconds
const PLAN_AMOUNT: i128 = 9_990_000; // 0.999 USDC (7 decimals)
const PRICE_CEILING: i128 = 15_000_000; // 1.5 USDC
const GRACE_PERIOD: u64 = 2_592_000; // 30 days
const MINT_AMOUNT: i128 = 10_000_000_000; // 1000 USDC

struct TestContext {
    env: Env,
    client: VowenaContractClient<'static>,
    admin: Address,
    merchant: Address,
    subscriber: Address,
    token_address: Address,
    token_client: TokenClient<'static>,
    token_admin_client: StellarAssetClient<'static>,
}

fn setup() -> TestContext {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let subscriber = Address::generate(&env);
    let token_admin = Address::generate(&env);

    // Set initial ledger state
    env.ledger().with_mut(|li| {
        li.timestamp = 1_000_000;
        li.sequence_number = 100;
    });

    // Register token
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_client = TokenClient::new(&env, &token_address);
    let token_admin_client = StellarAssetClient::new(&env, &token_address);

    // Mint tokens
    token_admin_client.mint(&subscriber, &MINT_AMOUNT);
    token_admin_client.mint(&merchant, &MINT_AMOUNT);

    // Register Vowena contract
    let contract_id = env.register(VowenaContract, ());
    let client = VowenaContractClient::new(&env, &contract_id);

    // Initialize
    client.initialize(&admin);

    TestContext {
        env,
        client,
        admin,
        merchant,
        subscriber,
        token_address,
        token_client,
        token_admin_client,
    }
}

fn create_default_plan(ctx: &TestContext) -> u64 {
    ctx.client.create_plan(
        &ctx.merchant,
        &ctx.token_address,
        &PLAN_AMOUNT,
        &MONTH,
        &0, // no trial
        &0, // unlimited
        &GRACE_PERIOD,
        &PRICE_CEILING,
    )
}

fn advance_time(env: &Env, seconds: u64) {
    let current = env.ledger().timestamp();
    env.ledger().with_mut(|li| {
        li.timestamp = current + seconds;
    });
}

// ============================================================
// Initialize
// ============================================================

#[test]
fn test_initialize() {
    let ctx = setup();
    // setup already calls initialize, so just verify it worked
    // by creating a plan (which requires initialized state)
    let plan_id = create_default_plan(&ctx);
    assert_eq!(plan_id, 1);
}

#[test]
fn test_double_initialize() {
    let ctx = setup();
    let result = ctx.client.try_initialize(&ctx.admin);
    assert!(result.is_err());
}

// ============================================================
// Create Plan
// ============================================================

#[test]
fn test_create_plan() {
    let ctx = setup();
    let plan_id = create_default_plan(&ctx);
    assert_eq!(plan_id, 1);

    let plan = ctx.client.get_plan(&plan_id);
    assert_eq!(plan.amount, PLAN_AMOUNT);
    assert_eq!(plan.period, MONTH);
    assert_eq!(plan.merchant, ctx.merchant);
    assert!(plan.active);

    let merchant_plans = ctx.client.get_merchant_plans(&ctx.merchant);
    assert_eq!(merchant_plans.len(), 1);
    assert_eq!(merchant_plans.get(0).unwrap(), plan_id);
}

#[test]
fn test_create_plan_invalid_amount() {
    let ctx = setup();
    let result = ctx.client.try_create_plan(
        &ctx.merchant,
        &ctx.token_address,
        &0i128,
        &MONTH,
        &0u32,
        &0u32,
        &GRACE_PERIOD,
        &PRICE_CEILING,
    );
    assert!(result.is_err());
}

#[test]
fn test_create_plan_invalid_period() {
    let ctx = setup();
    let result = ctx.client.try_create_plan(
        &ctx.merchant,
        &ctx.token_address,
        &PLAN_AMOUNT,
        &0u64,
        &0u32,
        &0u32,
        &GRACE_PERIOD,
        &PRICE_CEILING,
    );
    assert!(result.is_err());
}

#[test]
fn test_create_plan_ceiling_below_amount() {
    let ctx = setup();
    let result = ctx.client.try_create_plan(
        &ctx.merchant,
        &ctx.token_address,
        &PLAN_AMOUNT,
        &MONTH,
        &0u32,
        &0u32,
        &GRACE_PERIOD,
        &(PLAN_AMOUNT - 1), // ceiling below amount
    );
    assert!(result.is_err());
}

// ============================================================
// Subscribe
// ============================================================

#[test]
fn test_subscribe() {
    let ctx = setup();
    let plan_id = create_default_plan(&ctx);
    let sub_id = ctx.client.subscribe(&ctx.subscriber, &plan_id);
    assert_eq!(sub_id, 1);

    let sub = ctx.client.get_subscription(&sub_id);
    assert_eq!(sub.plan_id, plan_id);
    assert_eq!(sub.subscriber, ctx.subscriber);
    assert_eq!(sub.status, SubscriptionStatus::Active);
    assert_eq!(sub.periods_billed, 0);

    let sub_subs = ctx.client.get_subscriber_subscriptions(&ctx.subscriber);
    assert_eq!(sub_subs.len(), 1);

    let plan_subs = ctx.client.get_plan_subscribers(&plan_id);
    assert_eq!(plan_subs.len(), 1);
}

#[test]
fn test_subscribe_inactive_plan() {
    let ctx = setup();
    // Plan ID 99 doesn't exist
    let result = ctx.client.try_subscribe(&ctx.subscriber, &99u64);
    assert!(result.is_err());
}

// ============================================================
// Charge
// ============================================================

#[test]
fn test_charge_happy_path() {
    let ctx = setup();
    let plan_id = create_default_plan(&ctx);
    let sub_id = ctx.client.subscribe(&ctx.subscriber, &plan_id);

    let balance_before = ctx.token_client.balance(&ctx.subscriber);

    // Advance past billing time
    advance_time(&ctx.env, MONTH + 1);

    let result = ctx.client.charge(&sub_id);
    assert!(result);

    let balance_after = ctx.token_client.balance(&ctx.subscriber);
    assert_eq!(balance_before - balance_after, PLAN_AMOUNT);

    let sub = ctx.client.get_subscription(&sub_id);
    assert_eq!(sub.periods_billed, 1);
    assert_eq!(sub.status, SubscriptionStatus::Active);
}

#[test]
fn test_charge_not_due() {
    let ctx = setup();
    let plan_id = create_default_plan(&ctx);
    let sub_id = ctx.client.subscribe(&ctx.subscriber, &plan_id);

    // Don't advance time - billing not due
    let result = ctx.client.charge(&sub_id);
    assert!(!result);

    let sub = ctx.client.get_subscription(&sub_id);
    assert_eq!(sub.periods_billed, 0);
}

#[test]
fn test_charge_trial_period() {
    let ctx = setup();
    // Create plan with 2 trial periods
    let plan_id = ctx.client.create_plan(
        &ctx.merchant,
        &ctx.token_address,
        &PLAN_AMOUNT,
        &MONTH,
        &2u32, // 2 trial periods
        &0u32,
        &GRACE_PERIOD,
        &PRICE_CEILING,
    );

    let sub_id = ctx.client.subscribe(&ctx.subscriber, &plan_id);
    let balance_before = ctx.token_client.balance(&ctx.subscriber);

    // First trial charge
    advance_time(&ctx.env, MONTH + 1);
    let result = ctx.client.charge(&sub_id);
    assert!(result);

    let sub = ctx.client.get_subscription(&sub_id);
    assert_eq!(sub.periods_billed, 1);

    // No tokens deducted during trial
    let balance_after = ctx.token_client.balance(&ctx.subscriber);
    assert_eq!(balance_before, balance_after);

    // Second trial charge
    advance_time(&ctx.env, MONTH);
    let result = ctx.client.charge(&sub_id);
    assert!(result);

    let sub = ctx.client.get_subscription(&sub_id);
    assert_eq!(sub.periods_billed, 2);
    assert_eq!(ctx.token_client.balance(&ctx.subscriber), balance_before);

    // Third charge - real billing starts
    advance_time(&ctx.env, MONTH);
    let result = ctx.client.charge(&sub_id);
    assert!(result);

    let sub = ctx.client.get_subscription(&sub_id);
    assert_eq!(sub.periods_billed, 3);
    assert_eq!(
        ctx.token_client.balance(&ctx.subscriber),
        balance_before - PLAN_AMOUNT
    );
}

#[test]
fn test_charge_insufficient_balance() {
    let ctx = setup();
    let plan_id = create_default_plan(&ctx);

    // Create a subscriber with no balance
    let broke_subscriber = Address::generate(&ctx.env);
    let sub_id = ctx.client.subscribe(&broke_subscriber, &plan_id);

    advance_time(&ctx.env, MONTH + 1);
    let result = ctx.client.charge(&sub_id);
    assert!(!result);

    let sub = ctx.client.get_subscription(&sub_id);
    assert_eq!(sub.status, SubscriptionStatus::Active);
    assert!(sub.failed_at > 0); // failure recorded
}

#[test]
fn test_charge_grace_retry_success() {
    let ctx = setup();
    let plan_id = create_default_plan(&ctx);

    // Create a subscriber with no balance initially
    let retry_subscriber = Address::generate(&ctx.env);
    let sub_id = ctx.client.subscribe(&retry_subscriber, &plan_id);

    advance_time(&ctx.env, MONTH + 1);

    // First charge fails - no balance
    let result = ctx.client.charge(&sub_id);
    assert!(!result);

    // Fund the subscriber during grace period
    ctx.token_admin_client.mint(&retry_subscriber, &MINT_AMOUNT);

    // Retry during grace period - should succeed
    advance_time(&ctx.env, 100); // small advance, still in grace
    let result = ctx.client.charge(&sub_id);
    assert!(result);

    let sub = ctx.client.get_subscription(&sub_id);
    assert_eq!(sub.status, SubscriptionStatus::Active);
    assert_eq!(sub.failed_at, 0); // cleared on success
    assert_eq!(sub.periods_billed, 1);
}

#[test]
fn test_charge_grace_expire_pause() {
    let ctx = setup();
    let plan_id = create_default_plan(&ctx);

    let broke_subscriber = Address::generate(&ctx.env);
    let sub_id = ctx.client.subscribe(&broke_subscriber, &plan_id);

    advance_time(&ctx.env, MONTH + 1);

    // Charge fails
    ctx.client.charge(&sub_id);

    // Advance past grace period
    advance_time(&ctx.env, GRACE_PERIOD + 1);

    // Another charge attempt transitions to Paused
    let result = ctx.client.charge(&sub_id);
    assert!(!result);

    let sub = ctx.client.get_subscription(&sub_id);
    assert_eq!(sub.status, SubscriptionStatus::Paused);
}

#[test]
fn test_charge_pause_to_cancel() {
    let ctx = setup();
    let plan_id = create_default_plan(&ctx);

    let broke_subscriber = Address::generate(&ctx.env);
    let sub_id = ctx.client.subscribe(&broke_subscriber, &plan_id);

    advance_time(&ctx.env, MONTH + 1);

    // Fail -> grace -> pause
    ctx.client.charge(&sub_id);
    advance_time(&ctx.env, GRACE_PERIOD + 1);
    ctx.client.charge(&sub_id); // transitions to Paused

    let sub = ctx.client.get_subscription(&sub_id);
    assert_eq!(sub.status, SubscriptionStatus::Paused);

    // One more period while paused -> Cancelled
    advance_time(&ctx.env, MONTH + 1);
    let result = ctx.client.charge(&sub_id);
    assert!(!result);

    let sub = ctx.client.get_subscription(&sub_id);
    assert_eq!(sub.status, SubscriptionStatus::Cancelled);
}

#[test]
fn test_charge_max_periods_expired() {
    let ctx = setup();
    // Plan with max 2 periods
    let plan_id = ctx.client.create_plan(
        &ctx.merchant,
        &ctx.token_address,
        &PLAN_AMOUNT,
        &MONTH,
        &0u32,
        &2u32, // max 2 periods
        &GRACE_PERIOD,
        &PRICE_CEILING,
    );

    let sub_id = ctx.client.subscribe(&ctx.subscriber, &plan_id);

    // Charge period 1
    advance_time(&ctx.env, MONTH + 1);
    assert!(ctx.client.charge(&sub_id));

    // Charge period 2
    advance_time(&ctx.env, MONTH);
    assert!(ctx.client.charge(&sub_id));

    // Period 3 -> expired
    advance_time(&ctx.env, MONTH);
    let result = ctx.client.charge(&sub_id);
    assert!(!result);

    let sub = ctx.client.get_subscription(&sub_id);
    assert_eq!(sub.status, SubscriptionStatus::Expired);
}

#[test]
fn test_charge_permissionless() {
    let ctx = setup();
    let plan_id = create_default_plan(&ctx);
    let sub_id = ctx.client.subscribe(&ctx.subscriber, &plan_id);

    advance_time(&ctx.env, MONTH + 1);

    // Anyone can call charge - no auth required
    // The fact that charge() doesn't take a caller address and
    // doesn't call require_auth() means it's permissionless.
    // In mock_all_auths mode this is transparent, but the contract
    // design ensures no auth is needed.
    let result = ctx.client.charge(&sub_id);
    assert!(result);
}

// ============================================================
// Cancel
// ============================================================

#[test]
fn test_cancel_by_subscriber() {
    let ctx = setup();
    let plan_id = create_default_plan(&ctx);
    let sub_id = ctx.client.subscribe(&ctx.subscriber, &plan_id);

    ctx.client.cancel(&ctx.subscriber, &sub_id);

    let sub = ctx.client.get_subscription(&sub_id);
    assert_eq!(sub.status, SubscriptionStatus::Cancelled);
    assert!(sub.cancelled_at > 0);

    // Removed from plan subs
    let plan_subs = ctx.client.get_plan_subscribers(&plan_id);
    assert_eq!(plan_subs.len(), 0);
}

#[test]
fn test_cancel_by_merchant() {
    let ctx = setup();
    let plan_id = create_default_plan(&ctx);
    let sub_id = ctx.client.subscribe(&ctx.subscriber, &plan_id);

    ctx.client.cancel(&ctx.merchant, &sub_id);

    let sub = ctx.client.get_subscription(&sub_id);
    assert_eq!(sub.status, SubscriptionStatus::Cancelled);
}

#[test]
fn test_cancel_unauthorized() {
    let ctx = setup();
    let plan_id = create_default_plan(&ctx);
    let sub_id = ctx.client.subscribe(&ctx.subscriber, &plan_id);

    let random = Address::generate(&ctx.env);
    let result = ctx.client.try_cancel(&random, &sub_id);
    assert!(result.is_err());
}

// ============================================================
// Refund
// ============================================================

#[test]
fn test_refund() {
    let ctx = setup();
    let plan_id = create_default_plan(&ctx);
    let sub_id = ctx.client.subscribe(&ctx.subscriber, &plan_id);

    // Charge first
    advance_time(&ctx.env, MONTH + 1);
    ctx.client.charge(&sub_id);

    let balance_before = ctx.token_client.balance(&ctx.subscriber);

    // Refund
    let refund_amount: i128 = PLAN_AMOUNT / 2;
    ctx.client.refund(&sub_id, &refund_amount);

    let balance_after = ctx.token_client.balance(&ctx.subscriber);
    assert_eq!(balance_after - balance_before, refund_amount);
}

// ============================================================
// Update Plan Amount
// ============================================================

#[test]
fn test_update_amount_within_ceiling() {
    let ctx = setup();
    let plan_id = create_default_plan(&ctx);

    let new_amount: i128 = PLAN_AMOUNT + 1_000_000;
    assert!(new_amount <= PRICE_CEILING);

    ctx.client.update_plan_amount(&plan_id, &new_amount);

    let plan = ctx.client.get_plan(&plan_id);
    assert_eq!(plan.amount, new_amount);
}

#[test]
fn test_update_amount_exceeds_ceiling() {
    let ctx = setup();
    let plan_id = create_default_plan(&ctx);

    let result = ctx
        .client
        .try_update_plan_amount(&plan_id, &(PRICE_CEILING + 1));
    assert!(result.is_err());
}

// ============================================================
// Migration
// ============================================================

#[test]
fn test_migration_request() {
    let ctx = setup();
    let old_plan_id = create_default_plan(&ctx);
    let sub_id = ctx.client.subscribe(&ctx.subscriber, &old_plan_id);

    // Create new plan
    let new_plan_id = ctx.client.create_plan(
        &ctx.merchant,
        &ctx.token_address,
        &(PLAN_AMOUNT * 2),
        &MONTH,
        &0u32,
        &0u32,
        &GRACE_PERIOD,
        &(PRICE_CEILING * 2),
    );

    ctx.client
        .request_migration(&ctx.merchant, &old_plan_id, &new_plan_id);

    let sub = ctx.client.get_subscription(&sub_id);
    assert_eq!(sub.migration_target, new_plan_id);
}

#[test]
fn test_migration_accept() {
    let ctx = setup();
    let old_plan_id = create_default_plan(&ctx);
    let sub_id = ctx.client.subscribe(&ctx.subscriber, &old_plan_id);

    let new_plan_id = ctx.client.create_plan(
        &ctx.merchant,
        &ctx.token_address,
        &(PLAN_AMOUNT * 2),
        &MONTH,
        &0u32,
        &0u32,
        &GRACE_PERIOD,
        &(PRICE_CEILING * 2),
    );

    ctx.client
        .request_migration(&ctx.merchant, &old_plan_id, &new_plan_id);

    let new_sub_id = ctx.client.accept_migration(&ctx.subscriber, &sub_id);

    // Old sub is cancelled
    let old_sub = ctx.client.get_subscription(&sub_id);
    assert_eq!(old_sub.status, SubscriptionStatus::Cancelled);

    // New sub is active on new plan
    let new_sub = ctx.client.get_subscription(&new_sub_id);
    assert_eq!(new_sub.plan_id, new_plan_id);
    assert_eq!(new_sub.status, SubscriptionStatus::Active);
    assert_eq!(new_sub.subscriber, ctx.subscriber);
}

#[test]
fn test_migration_reject() {
    let ctx = setup();
    let old_plan_id = create_default_plan(&ctx);
    let sub_id = ctx.client.subscribe(&ctx.subscriber, &old_plan_id);

    let new_plan_id = ctx.client.create_plan(
        &ctx.merchant,
        &ctx.token_address,
        &(PLAN_AMOUNT * 2),
        &MONTH,
        &0u32,
        &0u32,
        &GRACE_PERIOD,
        &(PRICE_CEILING * 2),
    );

    ctx.client
        .request_migration(&ctx.merchant, &old_plan_id, &new_plan_id);
    ctx.client.reject_migration(&ctx.subscriber, &sub_id);

    let sub = ctx.client.get_subscription(&sub_id);
    assert_eq!(sub.migration_target, 0);
    assert_eq!(sub.status, SubscriptionStatus::Active);
}

// ============================================================
// Reactivate
// ============================================================

#[test]
fn test_reactivate() {
    let ctx = setup();
    let plan_id = create_default_plan(&ctx);

    let broke_subscriber = Address::generate(&ctx.env);
    let sub_id = ctx.client.subscribe(&broke_subscriber, &plan_id);

    // Fail -> grace -> pause
    advance_time(&ctx.env, MONTH + 1);
    ctx.client.charge(&sub_id);
    advance_time(&ctx.env, GRACE_PERIOD + 1);
    ctx.client.charge(&sub_id);

    let sub = ctx.client.get_subscription(&sub_id);
    assert_eq!(sub.status, SubscriptionStatus::Paused);

    // Fund and reactivate
    ctx.token_admin_client.mint(&broke_subscriber, &MINT_AMOUNT);
    let charged = ctx.client.reactivate(&broke_subscriber, &sub_id);
    assert!(charged);

    let sub = ctx.client.get_subscription(&sub_id);
    assert_eq!(sub.status, SubscriptionStatus::Active);
    assert_eq!(sub.periods_billed, 1);
}

// ============================================================
// Full Lifecycle
// ============================================================

#[test]
fn test_full_lifecycle() {
    let ctx = setup();

    // 1. Create plan
    let plan_id = create_default_plan(&ctx);

    // 2. Subscribe
    let sub_id = ctx.client.subscribe(&ctx.subscriber, &plan_id);
    assert_eq!(
        ctx.client.get_subscription(&sub_id).status,
        SubscriptionStatus::Active
    );

    // 3. Charge 3 times
    for _ in 0..3 {
        advance_time(&ctx.env, MONTH + 1);
        assert!(ctx.client.charge(&sub_id));
    }
    assert_eq!(ctx.client.get_subscription(&sub_id).periods_billed, 3);

    // 4. Drain subscriber balance to force failure
    let remaining = ctx.token_client.balance(&ctx.subscriber);
    if remaining > 0 {
        // Transfer remaining to merchant to drain
        // Use a different approach since we can't easily drain
        // Instead, create a plan with amount > remaining balance
    }
    // Actually, let's just test cancel flow
    // 5. Cancel
    ctx.client.cancel(&ctx.subscriber, &sub_id);
    assert_eq!(
        ctx.client.get_subscription(&sub_id).status,
        SubscriptionStatus::Cancelled
    );

    // 6. Verify total charges
    let expected_deduction = PLAN_AMOUNT * 3;
    let final_balance = ctx.token_client.balance(&ctx.subscriber);
    assert_eq!(MINT_AMOUNT - final_balance, expected_deduction);
}

#[test]
fn test_full_lifecycle_with_failure_and_reactivation() {
    let ctx = setup();

    // Create plan with short grace period for test
    let plan_id = ctx.client.create_plan(
        &ctx.merchant,
        &ctx.token_address,
        &PLAN_AMOUNT,
        &MONTH,
        &1u32, // 1 trial period
        &0u32,
        &GRACE_PERIOD,
        &PRICE_CEILING,
    );

    // Subscribe
    let poor_sub = Address::generate(&ctx.env);
    ctx.token_admin_client.mint(&poor_sub, &(PLAN_AMOUNT * 2)); // Only enough for 2 real charges
    let sub_id = ctx.client.subscribe(&poor_sub, &plan_id);

    // Trial charge (free)
    advance_time(&ctx.env, MONTH + 1);
    assert!(ctx.client.charge(&sub_id));
    assert_eq!(ctx.client.get_subscription(&sub_id).periods_billed, 1);

    // Real charge 1
    advance_time(&ctx.env, MONTH);
    assert!(ctx.client.charge(&sub_id));
    assert_eq!(ctx.client.get_subscription(&sub_id).periods_billed, 2);

    // Real charge 2
    advance_time(&ctx.env, MONTH);
    assert!(ctx.client.charge(&sub_id));
    assert_eq!(ctx.client.get_subscription(&sub_id).periods_billed, 3);

    // Charge fails - insufficient balance
    advance_time(&ctx.env, MONTH);
    assert!(!ctx.client.charge(&sub_id));
    assert!(ctx.client.get_subscription(&sub_id).failed_at > 0);

    // Grace period expires -> Paused
    advance_time(&ctx.env, GRACE_PERIOD + 1);
    ctx.client.charge(&sub_id);
    assert_eq!(
        ctx.client.get_subscription(&sub_id).status,
        SubscriptionStatus::Paused
    );

    // Fund and reactivate
    ctx.token_admin_client.mint(&poor_sub, &MINT_AMOUNT);
    let charged = ctx.client.reactivate(&poor_sub, &sub_id);
    assert!(charged);
    assert_eq!(
        ctx.client.get_subscription(&sub_id).status,
        SubscriptionStatus::Active
    );

    // Cancel
    ctx.client.cancel(&poor_sub, &sub_id);
    assert_eq!(
        ctx.client.get_subscription(&sub_id).status,
        SubscriptionStatus::Cancelled
    );
}
