# 0001 — Exact Rust and Bevy pins

- Status: accepted
- Date: 2026-10-03
- Issue: #35 (toolchain and Bevy pins), reaffirmed in #85 (Q7/Q8 context)

## Context

Builds must resolve identically on every machine and in CI. A floating
toolchain or engine version moves float codegen and the dependency tree
without any commit.

## Decision

Rust stable `1.97.1` exact in `rust-toolchain.toml`, edition 2024; Bevy
exact `=0.19.1` with `default-features = false`, declared once in
`[workspace.dependencies]`. Both change only through a new ADR
(`E-PIN-ADR`, `E-PIN-TOOLCHAIN`, `E-PIN-BEVY`).

## Consequences

Every build, including CI, uses the same compiler and engine. Upgrades are
deliberate, reviewed pull requests, never silent.
