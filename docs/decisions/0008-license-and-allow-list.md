# 0008 — License MIT OR Apache-2 point 0 and allow-list

- Status: accepted
- Date: 2026-10-03
- Issue: #85 (Q9 license choice by the owner)

## Context

The crates carried no `license` field and the repository had no `LICENSE`
file, so the dependency license check could not cover the workspace's own
crates.

## Decision

Dual license `MIT OR Apache-2.0` (owner choice, Q9): a `LICENSE` file at
the root (lands in M2), `license = "MIT OR Apache-2.0"` in
`[workspace.package]` inherited by every crate, `publish = false` kept.
The `cargo-deny` allow-list (lands in M2) accepts MIT and Apache-2.0, and
grows only by ADR (`E-DENY`, `E-WORKSPACE-META`).

## Consequences

The public repository is dual licensed from M2 onward, and the license
check enforces the allow-list on every pull request.
