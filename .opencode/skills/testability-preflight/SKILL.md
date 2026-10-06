---
name: testability-preflight
description: Use when confirming the validator can test an approved spec at workflow stage 5, before planning or implementation starts.
---

# Testability Preflight

## Inputs

- Approved spec (goal, acceptance criteria, test plan with human steps + expected results, testability needs).
- `docs/validator-capabilities.md` (registry of already-proven capabilities).

## Steps

1. List what the validator needs to run the test plan (test environment access, test accounts, starting data, third-party services) and compare against `../../../docs/validator-capabilities.md`. Write the gap list: needed versus already proven.
2. If every need is covered by a proven registry row, skip the run: post a `preflight skip` record on the Issue naming `docs/validator-capabilities.md` and the exact entries that cover the needs (a bare claim with no entry never counts; `python scripts/gates/spec_gate.py --issue N` refuses it). Otherwise continue below.
3. If anything only the owner can provide is missing, ask the owner in non-technical terms on the Issue and move the board status per `docs/board.md` (`--status-key blocked`). Secrets are never written in Issues, comments, or code.
4. Launch the validator subagent in preflight mode with only the approved spec and `docs/validator-capabilities.md`. There is no result yet. The validator must reach the starting state from the test plan and perform the first step that does not depend on the new work (for example, signing in and reaching the screen where the change will appear). The main agent's statement that access exists is not enough.
5. Post the preflight result as an Issue comment (pass or fail with reasons and evidence).
6. Update `docs/validator-capabilities.md` with any newly proven capability (capability, how proven, Issue, date).
7. Final step: record the output on the Issue and set Status via `python scripts/sidebar/project.py set-status` (verified by re-read) — stay `--status-key pickedUp` on a pass (or a valid skip), or `--status-key blocked` when waiting on the owner or on a separate capability Issue. End with the one-glance summary per `docs/board.md`.

## Outputs

- Gap list (needed versus already proven).
- Owner access request for anything only the owner can give, when needed.
- Validator preflight verdict posted as an Issue comment.
- `../../../docs/validator-capabilities.md` updated.

## Done criteria

- Done when a preflight pass — or a valid `preflight skip` record naming registry entries — is recorded; or the owner comments `owner-test-only approved`; or a separate capability work Issue is created and this item waits on it. Implementation never starts until one of these holds.
