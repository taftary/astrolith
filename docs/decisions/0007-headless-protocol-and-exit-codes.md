# 0007 — Headless verify capture protocol and exit codes

- Status: accepted
- Date: 2026-10-03
- Issue: #35 (`--verify` gate), #85 (Q7 exit code 2)

## Context

The automated acceptance gate needs a machine-readable contract: fixed
output lines, golden files, and exit codes that distinguish "a check
failed" from "the command line was wrong".

## Decision

`--verify` and `--capture` print the documented line protocol to stdout
(`E-STDOUT-PROTOCOL`) and exit 0 when every check passed, 1 when a check
failed, 2 when the command line itself was wrong (Q7; exit code 2 lands in
M5). The validator and the golden files depend on the exact formats, so
changing a line is a decision, not an edit.

## Consequences

Machines consume stdout; humans read logs. Exit code 2 (from M5) makes
usage mistakes honest without touching normal runs.
