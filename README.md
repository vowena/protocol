# Vowena Protocol

Trustless recurring payments on Stellar, powered by Soroban smart contracts.

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![CI](https://github.com/vowena/protocol/actions/workflows/ci.yml/badge.svg)](https://github.com/vowena/protocol/actions/workflows/ci.yml)
[![Built on Stellar](https://img.shields.io/badge/Built%20on-Stellar-black?logo=stellar)](https://stellar.org)

## What is this?

Vowena Protocol is a Soroban smart contract that enables trustless subscription billing on the Stellar network. Merchants create billing plans, subscribers approve token allowances via the SEP-41 token interface, and anyone can trigger charges when they come due. No intermediary holds funds. No centralized party controls the flow.

The contract handles the full subscription lifecycle: plan creation, subscribing, periodic charging, cancellation, refunds, plan migrations, trial periods, grace periods, and price bands.

## Features

- **Trustless billing.** Charges happen on-chain through token allowances. The contract never custodies subscriber funds.
- **Permissionless charging.** Anyone can call `charge()` on a due subscription. Merchants, bots, or third parties can trigger billing.
- **Trial periods.** Plans support configurable trial periods where no charge is made.
- **Grace periods.** Failed charges don't immediately cancel subscriptions. Subscribers get time to top up their balance.
- **Price bands.** Merchants set a price ceiling at plan creation. Prices can change within that ceiling without re-approval from subscribers.
- **Plan migrations.** Merchants can propose moving subscribers from one plan to another. Subscribers accept or reject.
- **Refunds.** Merchants can issue partial or full refunds directly through the contract.
- **Reactivation.** Paused subscriptions can be reactivated with a fresh token allowance.

## Architecture

The contract is organized into focused modules:

| Module         | Purpose                                                       |
| -------------- | ------------------------------------------------------------- |
| `contract.rs`  | Public entry points for all contract functions                |
| `types.rs`     | Core data types: `Plan`, `Subscription`, `SubscriptionStatus` |
| `storage.rs`   | Ledger storage helpers, TTL management, key definitions       |
| `billing.rs`   | Charge processing logic, trial and grace period handling      |
| `migration.rs` | Plan migration request, accept, and reject flows              |
| `events.rs`    | Structured event emission for indexers and UIs                |
| `errors.rs`    | `VowenaError` enum with all contract error codes              |

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Stellar CLI](https://developers.stellar.org/docs/tools/stellar-cli) v22+
- WASM target: `rustup target add wasm32-unknown-unknown`

## Build

```sh
stellar contract build
```

This compiles the contract to `target/wasm32-unknown-unknown/release/vowena.wasm`.

## Test

```sh
cargo test
```

## Deploy (testnet)

```sh
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/vowena.wasm \
  --network testnet
```

## Related repositories

- [vowena/sdk](https://github.com/vowena/sdk) - TypeScript SDK for interacting with the protocol
- [vowena/dashboard](https://github.com/vowena/dashboard) - Merchant and subscriber dashboard
- [Documentation](https://docs.vowena.xyz)

## Contributing

Contributions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md) before getting started.

## License

Licensed under the [Apache License 2.0](LICENSE).
