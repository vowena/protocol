#![allow(deprecated)]

use soroban_sdk::{Address, Env, Symbol};

pub fn emit_plan_created(env: &Env, plan_id: u64, merchant: &Address) {
    env.events().publish(
        (Symbol::new(env, "plan_created"), merchant.clone()),
        plan_id,
    );
}

pub fn emit_project_created(env: &Env, project_id: u64, merchant: &Address) {
    env.events().publish(
        (Symbol::new(env, "project_created"), merchant.clone()),
        project_id,
    );
}

pub fn emit_plan_amount_updated(env: &Env, plan_id: u64, new_amount: i128) {
    env.events()
        .publish((Symbol::new(env, "plan_updated"),), (plan_id, new_amount));
}

pub fn emit_subscription_created(env: &Env, sub_id: u64, plan_id: u64, subscriber: &Address) {
    env.events().publish(
        (Symbol::new(env, "sub_created"), subscriber.clone()),
        (sub_id, plan_id),
    );
}

pub fn emit_charge_success(env: &Env, sub_id: u64, amount: i128, subscriber: &Address) {
    env.events().publish(
        (Symbol::new(env, "charge_ok"), subscriber.clone()),
        (sub_id, amount),
    );
}

pub fn emit_charge_failed(env: &Env, sub_id: u64, subscriber: &Address) {
    env.events().publish(
        (Symbol::new(env, "charge_fail"), subscriber.clone()),
        sub_id,
    );
}

pub fn emit_subscription_cancelled(env: &Env, sub_id: u64, subscriber: &Address) {
    env.events()
        .publish((Symbol::new(env, "sub_cancel"), subscriber.clone()), sub_id);
}

pub fn emit_subscription_paused(env: &Env, sub_id: u64, subscriber: &Address) {
    env.events()
        .publish((Symbol::new(env, "sub_paused"), subscriber.clone()), sub_id);
}

pub fn emit_subscription_expired(env: &Env, sub_id: u64, subscriber: &Address) {
    env.events().publish(
        (Symbol::new(env, "sub_expired"), subscriber.clone()),
        sub_id,
    );
}

pub fn emit_subscription_reactivated(env: &Env, sub_id: u64, subscriber: &Address) {
    env.events()
        .publish((Symbol::new(env, "sub_react"), subscriber.clone()), sub_id);
}

pub fn emit_refund_issued(env: &Env, sub_id: u64, amount: i128, subscriber: &Address) {
    env.events().publish(
        (Symbol::new(env, "refund"), subscriber.clone()),
        (sub_id, amount),
    );
}

pub fn emit_migration_requested(env: &Env, old_plan_id: u64, new_plan_id: u64) {
    env.events()
        .publish((Symbol::new(env, "mig_req"),), (old_plan_id, new_plan_id));
}

pub fn emit_migration_accepted(env: &Env, old_sub_id: u64, new_sub_id: u64, subscriber: &Address) {
    env.events().publish(
        (Symbol::new(env, "mig_accept"), subscriber.clone()),
        (old_sub_id, new_sub_id),
    );
}

pub fn emit_migration_rejected(env: &Env, sub_id: u64, subscriber: &Address) {
    env.events()
        .publish((Symbol::new(env, "mig_reject"), subscriber.clone()), sub_id);
}
