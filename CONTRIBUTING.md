# Contributing to Vowena Protocol

Thanks for your interest in contributing. This repository contains billing logic that can affect real money flows, so the standard here is deliberately high: every change should be explainable, testable, and safe to review.

## Good first contributions

- Missing test coverage
- Documentation improvements
- Internal refactors with no behavior change
- Tooling and CI improvements
- Small bug fixes with a clear reproduction

Start by reading [README.md](README.md), [SUPPORT.md](SUPPORT.md), and [SECURITY.md](SECURITY.md).

## Prerequisites

- Rust stable: <https://rustup.rs/>
- WASM target: `rustup target add wasm32-unknown-unknown`
- Stellar CLI v22+ if you want to use `stellar contract build`

## Local setup

```sh
git clone https://github.com/vowena/protocol.git
cd protocol
rustup target add wasm32-unknown-unknown
cargo test
cargo build --target wasm32-unknown-unknown --release
```

If you use the Stellar CLI locally, `stellar contract build` is also fine.

## Development workflow

1. Fork the repository.
2. Create a focused branch from `main`.
3. Make the smallest change that fully solves the problem.
4. Add or update tests.
5. Run the full verification suite before opening a PR.

## Branch naming

- `feat/batch-charge`
- `fix/grace-period-overflow`
- `docs/update-deployment-guide`
- `refactor/extract-storage-helpers`
- `test/add-refund-edge-cases`
- `chore/upgrade-soroban-sdk`

## Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org/):

```text
feat: add merchant-owned charge guard
fix: prevent retry after terminal cancellation
docs: clarify migration acceptance flow
refactor: extract ttl helper utilities
test: cover zero-amount refund rejection
chore: tighten CI checks
```

## Verification

Run all of these before you push:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
cargo build --target wasm32-unknown-unknown --release --locked
```

## Code expectations

- Preserve protocol invariants and spell them out in the PR when behavior changes.
- Keep storage layout changes explicit and well documented.
- Avoid hidden behavior changes inside refactors.
- Follow the existing module boundaries unless the change clearly improves reviewability.
- The contract is `#![no_std]`; do not introduce `std`.

## Testing expectations

- New behavior needs tests.
- Bug fixes need regression coverage.
- When adding failure paths, assert the exact error where practical.
- If a change touches billing cadence, refunds, migrations, or authorization, call that out clearly in the PR description.

## Pull requests

- Fill out the PR template.
- Link the related issue when there is one.
- Explain the invariant or user-facing behavior being changed.
- Note any migration, release, or deployment implications.
- Keep PRs focused enough to review thoroughly.

## Security

Never use public issues for vulnerabilities. Follow [SECURITY.md](SECURITY.md).

## Conduct

This repository follows [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).
