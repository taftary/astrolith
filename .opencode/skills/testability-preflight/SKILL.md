---
name: testability-preflight
description: Use when confirming the validator can test an approved spec at workflow stage 5, before planning or implementation starts.
---

# Testability Preflight

## Inputs

- Approved spec (goal, acceptance criteria, test plan with human steps + expected results, testability needs).
- `../../../docs/validator-capabilities.md` (registry of already-proven capabilities).

## Steps

1. List what the validator needs to run the test plan (test environment access, test accounts, starting data, third-party services) and compare against `../../../docs/validator-capabilities.md`. Write the gap list: needed versus already proven.
2. If anything only the owner can provide is missing, ask the owner in non-technical terms on the Issue and move the board status per `../../../docs/board.md` to `Needs your answer`. Secrets are never written in Issues, comments, or code.
3. Launch the validator subagent in preflight mode with only the approved spec and `../../../docs/validator-capabilities.md`. There is no result yet. The validator must reach the starting state from the test plan and perform the first step that does not depend on the new work (for example, signing in and reaching the screen where the change will appear). The main agent's statement that access exists is not enough.
4. Post the preflight result as an Issue comment (pass or fail with reasons and evidence).
5. Update `../../../docs/validator-capabilities.md` with any newly proven capability (capability, how proven, Issue, date).
6. Final step: record the output on the Issue and move the board status per `../../../docs/board.md` — stay in `Working` on a pass, or `Needs your answer` when waiting on the owner or on a separate capability Issue.

## Outputs

- Gap list (needed versus already proven).
- Owner access request for anything only the owner can give, when needed.
- Validator preflight verdict posted as an Issue comment.
- `../../../docs/validator-capabilities.md` updated.

## Done criteria

- Done when a preflight pass is recorded; or the owner comments `owner-test-only approved`; or a separate capability work Issue is created and this item waits on it. Implementation never starts until one of these holds.
