# 0056: Opt-in real-input scenarios

Status: accepted

Issue: #1227 (Spec v2).

## Context

Camera-pose routes cannot prove player movement or UI interaction. The validator
needs a bounded public interface without desktop automation or fabricated state.

## Decision

Add `--scenario FILE NEW_DIRECTORY`, separate from routes. The versioned
`scenario-v1` text defines fixed-seed preparation followed by ordinary input
events and read-only assertions. Setup may initialize real gameplay state;
actions may not call gameplay handlers or write player/UI state directly.

The app streams tab-separated evidence and finishes with `SCENARIO-OK` or
`SCENARIO-FAIL <reason>`. Exit codes are 0/1 for outcomes and 2 for bad CLI usage.
Existing headless/route protocols, dependencies and pins do not change.

## Consequences

Normal startup installs no scenario systems or collectors. A dedicated runner
supplies disposable settings, executable/fixture identity and an outer timeout.
Incomplete runs never succeed. Near-moon preparation is explicitly hovering,
not moon landing or collision support. Future feature-specific observations
must read actual application state, not expected values from the test file.
