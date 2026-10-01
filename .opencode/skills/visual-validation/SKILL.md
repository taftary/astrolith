---
name: visual-validation
description: Load when a pull request contains any user-visible UI change (part of workflow stage 10). Use to check rendered UI against the spec on the PR preview.
---

# Visual Validation

Required for any user-visible UI change, optional otherwise. Runs inside `technical-validation` before the validator gate, per `docs/workflow.md` (Validation Layers).

## Inputs

- PR preview URL (or test environment URL) at the head commit SHA.
- Approved spec: test plan steps + expected results for every user-visible change.

## Steps

1. **Check each change.** For every user-visible change, perform the spec's human steps on the PR preview and compare what is seen against the spec's expected results.
2. **Post evidence on the PR.** For each check: step taken, observed result (text seen, screenshot/log reference where available), pass/fail against expected. Name the commit SHA checked.
3. **Handle discrepancies.** Fix small discrepancies on the branch directly (new commit, then re-check that item); list any remaining discrepancies explicitly on the PR so `technical-validation` does not proceed silently.
4. **Record and move status.** Reference the PR evidence comment from the parent Issue. No status change; item stays In progress (confirmed with `python scripts/sidebar/project.py get --issue N`).

## Outputs

- Evidence comment on the PR per user-visible change, naming the SHA.

## Done criteria

- Every user-visible change checked against the spec's expected results on the PR preview.
- Evidence posted on the PR; discrepancies fixed (re-checked) or explicitly listed.
- Output referenced on the Issue and status handled per `docs/board.md`.
