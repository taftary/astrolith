---
name: technical-validation
description: Load when technically validating an open pull request (workflow stage 10). Use to check CI and tests, run visual validation for UI changes, and obtain the validator gate verdict.
---

# Technical Validation

Main-agent self-check on the PR before the validator gate, per `docs/workflow.md` (Validation Layers) and `docs/plans/workflow.md` skills table. Does not replace the validator gate or owner testing.

## Inputs

- Open PR number and head commit SHA.
- Parent Issue number `#N` with original notion verbatim and approved spec (goal, acceptance criteria, test plan, testability needs).

## Steps

1. **CI + tests + runtime.** Read CI status (`ci` job) and test results for the head SHA. Then run the runtime validator yourself (it launches the real app, never mocks the thing under test):
   - `python scripts/validation/validate.py --issue N --sha <head-sha>`
   - It builds, starts the app with real env/migrations/seed data, waits for a real health check (not just "process started"), exercises the feature end to end plus one edge/error path, and scans ALL logs (server stdout/stderr, browser console where applicable). Any error fails unless the owner allowlisted it.
   - Summarize all three (CI, tests, runtime) in a comment on the PR (what ran, pass/fail, links). If red, fix on the branch (new commit) and restart this skill for the new SHA.
2. **Visual validation.** If the PR contains any user-visible UI change, run the `visual-validation` skill now and require its evidence before continuing. If no UI change, note that explicitly on the PR.
3. **Launch the validator** (subagent `validator`, fresh context, it never writes code). Give it exactly three inputs and nothing else:
   - Original notion (verbatim from the Issue).
   - Approved specification including test plan and testability needs.
   - Result: PR preview / test environment URL + commit SHA.
   - It tests only through the public interface and flags spec-vs-notion drift.
4. **Confirm the verdict on the Issue.** Require the validator's machine-readable marker plus human-readable verdict for the head SHA:
   - `<!-- validator:pass sha=<full-sha> -->` or `<!-- validator:fail sha=<full-sha> -->`
   - `Validator: PASS | FAIL for <sha>` with Checked, Evidence, Reasons (on fail), Drift from notion.
   - Verify the marker SHA equals the PR head SHA. On FAIL, or on drift (`Drift from notion` is not `none`), set Status to In progress (`python scripts/sidebar/project.py set-status --issue N --status "In progress"`) and return to correction: do not merge. Maximum 3 validator loops; after that set Status to Needs your answer and send the owner the full report.
5. **Record and move status.** Post validation summary on the Issue (CI result, test result, runtime result, validator verdict link + SHA). On validator PASS with the PR open, move to In review (`python scripts/sidebar/project.py set-status --issue N --status "In review"`); on FAIL, to In progress. Never move to In review without a validator PASS for the exact head SHA. End with the one-glance summary: Project/Status, Milestone, Relationships, Linked branches/PRs, Subscription, Last validation verdict (read each with the `scripts/sidebar/` tools).

## Outputs

- CI/test/runtime summary comment on the PR.
- Validator verdict comment on the Issue naming the head SHA.

## Done criteria

- CI green for the head SHA and results posted on the PR.
- Runtime validation clean (app launched, health check passed, logs scanned, edge case covered).
- Visual validation done or explicitly not needed.
- Validator verdict recorded on the Issue for the exact head SHA (pass required before the `pull-request` skill may merge).
- Output recorded on the Issue and status moved per `docs/board.md`.
