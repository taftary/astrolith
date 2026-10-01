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

1. **CI + tests.** Read CI status (`ci` job) and test results for the head SHA. Summarize them in a comment on the PR (what ran, pass/fail, links). If red, fix on the branch (new commit) and restart this skill for the new SHA.
2. **Visual validation.** If the PR contains any user-visible UI change, run the `visual-validation` skill now and require its evidence before continuing. If no UI change, note that explicitly on the PR.
3. **Launch the validator** (subagent `validator`, fresh context, it never writes code). Give it exactly three inputs and nothing else:
   - Original notion (verbatim from the Issue).
   - Approved specification including test plan and testability needs.
   - Result: PR preview / test environment URL + commit SHA.
   - It tests only through the public interface and flags spec-vs-notion drift.
4. **Confirm the verdict on the Issue.** Require the validator's machine-readable marker plus human-readable verdict for the head SHA:
   - `<!-- validator:pass sha=<full-sha> -->` or `<!-- validator:fail sha=<full-sha> -->`
   - `Validator: PASS | FAIL for <sha>` with Checked, Evidence, Reasons (on fail), Drift from notion.
   - Verify the marker SHA equals the PR head SHA. On FAIL, or on drift (`Drift from notion` is not `none`), return to correction: do not merge.
5. **Record and move status.** Post validation summary on the Issue (CI result, test result, validator verdict link + SHA). Move board status per `docs/board.md` (normally stays Working until merge).

## Outputs

- CI/test summary comment on the PR.
- Validator verdict comment on the Issue naming the head SHA.

## Done criteria

- CI green for the head SHA and results posted on the PR.
- Visual validation done or explicitly not needed.
- Validator verdict recorded on the Issue for the exact head SHA (pass required before the `pull-request` skill may merge).
- Output recorded on the Issue and status moved per `docs/board.md`.
