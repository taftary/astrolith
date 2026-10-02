---
name: visual-validation
description: Load when a pull request contains any user-visible UI change (part of workflow stage 10). Use to check rendered UI against the spec on the PR preview.
---

# Visual Validation

Required for any user-visible UI change, optional otherwise. Runs inside `technical-validation` before the validator gate, per `docs/workflow.md` (Validation Layers). This product is a native window: there is no PR preview URL. Run the app at the head SHA with `--capture <dir>` (scripted input sequence writing frames plus a snapshot log), or, when the change cannot be captured, record per-item `owner-test-only` on the Issue and skip the rest.

## Inputs

- Head commit SHA plus `--capture <dir>` output (frames + snapshot log), or the `owner-test-only` record link.
- Approved spec: test plan steps + expected results for every user-visible change.

## Steps

1. **Check each change.** For every user-visible change, perform the spec's human steps against the capture output and compare what is seen against the spec's expected results.
2. **Post evidence on the PR.** For each check: step taken, observed result (frame reference, snapshot-log excerpt), pass/fail against expected. Name the commit SHA checked.
3. **Handle discrepancies.** Fix small discrepancies on the branch directly (new commit, then re-check that item); list any remaining discrepancies explicitly on the PR so `technical-validation` does not proceed silently.
4. **Record and move status.** Reference the PR evidence comment from the parent Issue. This skill moves no status on its own; the transition is handled by `technical-validation`. End with the one-glance summary per `docs/board.md`.

## Outputs

- Evidence comment on the PR per user-visible change, naming the SHA.

## Done criteria

- Every user-visible change checked against the spec's expected results via `--capture` output (or `owner-test-only` recorded on the Issue).
- Evidence posted on the PR; discrepancies fixed (re-checked) or explicitly listed.
- Output referenced on the Issue and status handled per `docs/board.md`.
