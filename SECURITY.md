# Security Policy

Vowena Protocol handles recurring billing logic on-chain. Vulnerabilities here can affect funds, authorization, and contract safety, so private reporting is required.

## Report a vulnerability

Do not open a public issue or pull request for a security finding.

Preferred reporting path:

1. Open a private GitHub security advisory: <https://github.com/vowena/protocol/security/advisories/new>
2. Or email `security@vowena.xyz` with the subject `[SECURITY] protocol vulnerability report`

## What to include

- A clear description of the issue
- Affected methods, modules, or invariants
- Steps to reproduce or a proof of concept
- Expected impact, especially any risk to funds or permissions
- Suggested mitigation if you have one

## Response targets

- Acknowledgment within 48 hours
- Initial assessment within 7 days
- Ongoing coordination until the issue is resolved

## In scope

- Authorization bypasses
- Incorrect token transfers or fund accounting
- Billing cadence, grace period, or trial period flaws
- Storage corruption or unsafe upgrade/migration behavior
- Event or state inconsistencies that can mislead integrations

## Out of scope

- Hypothetical findings with no plausible exploit path
- Vulnerabilities that only exist in upstream dependencies and should be reported there first

## Coordinated disclosure

Please give us time to validate, fix, and communicate safely before public disclosure.
