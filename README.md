<div align="center">

<a href="https://vowena.xyz">
  <img src="./.github/banner.svg" alt="Vowena Protocol" width="100%" />
</a>

<p>
  <strong>Trustless recurring payments on Stellar, powered by Soroban smart contracts.</strong>
</p>

<p>
  <a href="LICENSE"><img alt="License: Apache 2.0" src="https://img.shields.io/badge/license-Apache%202.0-6B4EFF?style=flat-square&labelColor=14141C"></a>
  <a href="https://github.com/vowena/protocol/actions/workflows/ci.yml"><img alt="CI" src="https://img.shields.io/github/actions/workflow/status/vowena/protocol/ci.yml?branch=main&style=flat-square&label=ci&labelColor=14141C&color=6B4EFF"></a>
  <a href="https://github.com/vowena/protocol/actions/workflows/ci.yml"><img alt="Tests" src="https://img.shields.io/badge/tests-29%20passing-00DC82?style=flat-square&labelColor=14141C"></a>
  <a href="https://stellar.expert/explorer/testnet/contract/CCNDNEGYFYKTVBM7T2BEF5YVSKKICE44JOVHT7SAN5YTKHHBFIIEL72T"><img alt="Soroban testnet" src="https://img.shields.io/badge/soroban-testnet-6B4EFF?style=flat-square&labelColor=14141C"></a>
  <a href="https://docs.vowena.xyz"><img alt="Docs" src="https://img.shields.io/badge/docs-vowena.xyz-B5A8FF?style=flat-square&labelColor=14141C"></a>
</p>

<p>
  <a href="https://docs.vowena.xyz">Documentation</a>
  &nbsp;·&nbsp;
  <a href="https://vowena.xyz">Website</a>
  &nbsp;·&nbsp;
  <a href="https://github.com/vowena/sdk">SDK</a>
  &nbsp;·&nbsp;
  <a href="https://dashboard.vowena.xyz">Dashboard</a>
</p>

</div>

<br />

## What is Vowena Protocol

Vowena Protocol is a single, shared Soroban smart contract that brings subscription billing to the Stellar network. There is one canonical instance on chain, and any merchant address can use it directly. Merchants create projects and plans, subscribers approve token allowances under the SEP-41 interface, and anyone can trigger a charge when one becomes due. Funds move directly between subscriber and merchant. The contract never custodies value.

The contract handles the full subscription lifecycle: project and plan creation, signup with an upfront charge or trial, periodic billing, grace periods, cancellation, refunds, plan migrations, reactivation, and price bands that let merchants adjust amounts within an authorized ceiling without requiring re-approval from subscribers.

<br />

## Quick start

```sh
# Add the WebAssembly target once
rustup target add wasm32-unknown-unknown

# Build the contract
stellar contract build

# Run the test suite
cargo test
```

The compiled artifact lands at `target/wasm32-unknown-unknown/release/vowena.wasm`.

<br />

## Function reference

The contract exposes 20 entry points, grouped by responsibility.

### Setup

| Function     | Auth  | Description                                                              |
| :----------- | :---- | :----------------------------------------------------------------------- |
| `initialize` | once  | Bind the contract to an admin address and seed ID counters. One-shot.    |

### Projects

| Function                | Auth     | Description                                                       |
| :---------------------- | :------- | :---------------------------------------------------------------- |
| `create_project`        | merchant | Create a project. Projects group plans under a single merchant.   |
| `get_project`           | none     | Fetch a single project by ID.                                     |
| `get_merchant_projects` | none     | List every project ID owned by a merchant address.                |

### Plans

| Function             | Auth     | Description                                                                  |
| :------------------- | :------- | :--------------------------------------------------------------------------- |
| `create_plan`        | merchant | Create a billing plan inside a project. Sets price ceiling and trial config. |
| `update_plan_amount` | merchant | Change the recurring amount within the plan's price ceiling.                 |
| `get_plan`           | none     | Fetch a single plan by ID.                                                   |
| `get_merchant_plans` | none     | List every plan ID owned by a merchant address.                              |

### Subscriptions

| Function                       | Auth       | Description                                                                                       |
| :----------------------------- | :--------- | :------------------------------------------------------------------------------------------------ |
| `subscribe`                    | subscriber | Approve allowance, create subscription, charge on signup unless the plan has a trial.             |
| `cancel`                       | either     | Cancel a subscription. Caller must be the subscriber or the plan merchant.                        |
| `reactivate`                   | subscriber | Re-approve allowance and resume a paused subscription. Attempts an immediate charge.              |
| `get_subscription`             | none       | Fetch a single subscription by ID.                                                                |
| `get_subscriber_subscriptions` | none       | List every subscription ID owned by a subscriber address.                                         |
| `get_plan_subscribers`         | none       | List every active subscription ID under a given plan.                                             |

### Billing

| Function | Auth  | Description                                                                                       |
| :------- | :---- | :------------------------------------------------------------------------------------------------ |
| `charge` | none  | Permissionless. Anyone may call. Pulls funds when the subscription is due and within allowance.   |
| `refund` | merchant | Issue a partial or full refund directly through the contract.                                  |

### Migrations

| Function            | Auth       | Description                                                                          |
| :------------------ | :--------- | :----------------------------------------------------------------------------------- |
| `request_migration` | merchant   | Propose moving subscribers from one plan to another. Both plans must belong to the merchant. |
| `accept_migration`  | subscriber | Accept the proposed migration. Re-approves allowance against the new plan.           |
| `reject_migration`  | subscriber | Decline the proposed migration. The subscription remains on the current plan.        |

### Utilities

| Function     | Auth | Description                                                                       |
| :----------- | :--- | :-------------------------------------------------------------------------------- |
| `extend_ttl` | none | Bump persistent storage TTL for a plan and subscription. Anyone may call.         |

<br />

## Architecture

Vowena is intentionally one shared contract. Merchants do not deploy their own instance and subscribers never need to learn a new contract address per merchant. State is partitioned by chain-assigned IDs and indexed by merchant or subscriber address, so the protocol scales horizontally without privileged tenancy.

### On-chain primitives

| Primitive      | Owner       | Purpose                                                                                |
| :------------- | :---------- | :------------------------------------------------------------------------------------- |
| `Project`      | merchant    | Top-level grouping. Holds display metadata for a related family of plans.              |
| `Plan`         | merchant    | Pricing template. Defines token, amount, period, trial, grace, max periods, ceiling.   |
| `Subscription` | subscriber  | Active billing relationship between a subscriber and a plan. Tracks status and cycles. |

### Storage strategy

The contract uses Soroban's persistent storage for all long-lived entities and instance storage for counters and the admin address. Every write extends the entity's TTL. The protocol exposes `extend_ttl` so external callers (or the dashboard) can keep entries warm without privileged access. TTL thresholds and extension windows are tuned for a 30 to 120 day persistence horizon.

### Source layout

| Module         | Responsibility                                                            |
| :------------- | :------------------------------------------------------------------------ |
| `contract.rs`  | Public entry points. Auth checks. Dispatch to billing and migration.      |
| `types.rs`     | `Project`, `Plan`, `Subscription`, `SubscriptionStatus`.                  |
| `storage.rs`   | Ledger keys, persistence helpers, TTL management.                         |
| `billing.rs`   | Charge processing. Trial accounting. Grace period and pause transitions. |
| `migration.rs` | Request, accept, and reject flows for plan migrations.                    |
| `events.rs`    | Structured event emission for indexers and UIs.                           |
| `errors.rs`    | `VowenaError` variants and contract error codes.                          |

<br />

## Deployments

### Testnet

```text
CCNDNEGYFYKTVBM7T2BEF5YVSKKICE44JOVHT7SAN5YTKHHBFIIEL72T
```

Live on the Stellar testnet. Inspect on [Stellar Expert](https://stellar.expert/explorer/testnet/contract/CCNDNEGYFYKTVBM7T2BEF5YVSKKICE44JOVHT7SAN5YTKHHBFIIEL72T).

### Mainnet

Coming soon. Mainnet deployment will follow the pilot program. Track the launch at [vowena.xyz](https://vowena.xyz).

<br />

## Local development

```sh
# Prerequisites
rustup target add wasm32-unknown-unknown
# Stellar CLI v22+ — see https://developers.stellar.org/docs/tools/stellar-cli

# Build
stellar contract build

# Test
cargo test

# Lint and format
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings

# Deploy to testnet (after `stellar keys generate` and `stellar network add testnet`)
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/vowena.wasm \
  --network testnet
```

The repository ships with a CI workflow that runs `fmt`, `clippy`, the full test suite, and a release WASM build on every pull request.

<br />

## Related projects

| Repository                                                  | Purpose                                                       |
| :---------------------------------------------------------- | :------------------------------------------------------------ |
| [`vowena/sdk`](https://github.com/vowena/sdk)               | TypeScript SDK. Typed bindings, helpers, transaction builders. |
| [`vowena/dashboard`](https://github.com/vowena/dashboard)   | Merchant and subscriber dashboard. Next.js application.        |
| [`vowena/docs`](https://github.com/vowena/docs)             | Public documentation source. Mintlify.                         |
| [`vowena/site`](https://github.com/vowena/site)             | Marketing site at vowena.xyz.                                  |

<br />

## Documentation

Full protocol reference, integration guides, and conceptual overviews live at [docs.vowena.xyz](https://docs.vowena.xyz).

<br />

## Contributing

Pull requests are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md) and the [Code of Conduct](CODE_OF_CONDUCT.md) before opening one. Security reports should follow [SECURITY.md](SECURITY.md).

<br />

## License

Licensed under the [Apache License, Version 2.0](LICENSE).

<br />

<div align="center">
  <sub>Built for the Stellar ecosystem.</sub>
</div>
