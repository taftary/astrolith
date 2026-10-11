---
name: technical-validation
description: Load when technically validating an open pull request (workflow stage 10). Use to check CI and tests, run visual validation for UI changes, and obtain the validator gate verdict.
---

# Technical Validation

Main-agent self-check on the PR before the validator gate, per `docs/workflow.md` (Validation Layers) and the stage-to-skill table in `docs/workflow.md` (Skills). Does not replace the validator gate or owner testing.

## Inputs

- Open PR number and head commit SHA.
- Parent Issue number `#N` with original notion verbatim and approved spec (goal, acceptance criteria, test plan, testability needs).

## Steps

1. **CI + cheap checks.** Read CI status (`ci` job) and test results for the head SHA. Then run only the cheap checks yourself (`cargo test --locked --workspace` and one headless app run `cargo run --locked -p universe-app -- --verify`: exit 0 + VERIFY-OK, together about 15 seconds) so a broken build is caught early and cheaply. The full validation script (`python scripts/validation/validate.py --issue N --sha <head-sha> --visual yes|no`, where `--visual` states what the approved spec claims: `yes` when it names a user-visible change) runs exactly once per round, by the validator in step 3 — not here. It covers the repeat-run determinism edge probe, the frame proof step (after/before captures with base reuse, picture publish, `Visual:` line, or the degraded `Visual: BLOCKED (infra: ...)` record when all four AC4 conditions hold), and scans ALL logs for errors. Web-style checks (HTTP health, browser console) apply only when the product has them. Any error fails unless the owner allowlisted it.
   - Summarize all three (CI, cheap checks) in a comment on the PR (what ran, pass/fail, links). If red, fix on the branch (new commit) and restart this skill for the new SHA.
2. **Visual validation.** If the PR contains any user-visible UI change, run the `visual-validation` skill now and require its evidence before continuing. If no UI change, note that explicitly on the PR.
3. **Launch the validator** (subagent `validator`, fresh context, it never writes code). First confirm a clean tracked tree at the PR head (`git status --porcelain --untracked-files=no` empty — untracked files are not part of any SHA and cannot void a pass — `git rev-parse HEAD` equals the PR head SHA); the validator re-confirms both as its first action. Give it exactly three inputs by link and nothing else:
   - Original notion: link the verbatim notion comment on the Issue.
   - Approved specification: link the approved `Spec v<k>` comment (a new version voids the old approval).
   - Result: PR number + head commit SHA (there is no preview URL for this native-window product; the validator runs headless `--verify` and `--capture` evidence).
   - If any input is missing or empty, the validator immediately reports BLOCKED naming the missing input, without executing.
   - It tests only through the public interface, never reads the branch diff, and flags spec-vs-notion drift.
4. **Confirm the verdict on the Issue.** Require the validator's machine-readable marker plus human-readable verdict for the head SHA:
   - `<!-- validator:pass sha=<full-sha> -->` or `<!-- validator:fail sha=<full-sha> -->`
   - `Validator: PASS | FAIL for <sha>` with Checked, Evidence, Reasons (on fail), Drift from notion, plus `Runtime: PASS`, `Requirements: MET n/n`, `Drift: none`, and the `Visual:` line with the pictures (the `frame-proof.md` fragment pasted below a blank line after the marker) the merge gate reads. The verdict comment is the validator's own output pasted verbatim (compared by `compare_verdict.py`); never rewrite it.
   - Verify the marker SHA equals the PR head SHA. On FAIL, or on drift, set `--status-key validatorFail` (always In progress, including correction rounds) and return to correction: do not merge. The bound in `docs/workflow.md` (Correction) is two consecutive validator fails; after that set `--status-key blocked` and send the owner the full report.
5. **Record and hand off.** Post validation summary on the Issue (CI result, cheap-check result, validator verdict link + SHA, and a note that the full script ran exactly once, by the validator). Include a "rule compliance" line listing any `E-` IDs from `docs/engineering.md` the change touches and how each was checked. On PASS the item stays In progress: hand to the `pull-request` skill for the merge gate and merge (In review is entered only on merge, never on PR open). End with the one-glance summary per `docs/board.md`.

## Outputs

- CI/test/runtime summary comment on the PR.
- Validator verdict comment on the Issue naming the head SHA.

## Done criteria

- CI green for the head SHA and results posted on the PR.
- Runtime validation clean (app launched, health check passed, logs scanned, edge case covered).
- Visual validation done or explicitly not needed.
- Validator verdict recorded on the Issue for the exact head SHA (pass required before the `pull-request` skill may merge).
- Output recorded on the Issue and status moved per `docs/board.md`.
