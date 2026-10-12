---
name: visual-validation
description: Load when a pull request contains any user-visible UI change (part of workflow stage 10). Use to check rendered UI against the spec on the PR preview.
---

# Visual Validation

Required for any user-visible UI change, optional otherwise. Runs inside `technical-validation` before the validator gate, per `docs/workflow.md` (Validation Layers). This product is a native window: there is no PR preview URL. Evidence is real-window screenshots from `cargo run --locked -p universe-app -- --screens <dir>` (the shot list in `crates/universe-render/src/screens.rs`), taken by the main agent at the head commit. The validator takes no pictures (#533), so this check and the owner test are the only eyes on the window. When the change cannot be captured, record per-item `owner-test-only` on the Issue and skip the rest.

## Inputs

- Head commit SHA plus `--screens <dir>` output (PNG shots + manifest), or the `owner-test-only` record link.
- Approved spec: test plan steps + expected results for every user-visible change.

## Steps

1. **Check each change.** For every user-visible change, look at the matching shots and compare what is seen against the spec's expected results. Add a shot to `screens.rs` when no existing one shows the change.
2. **Post the evidence.** Upload the relevant shots to the `validation-evidence` release and post them on the PR with one line each (shot, what it shows, matches the spec or not), naming the commit SHA checked. Reuse the best shots in the owner test steps.
3. **Handle discrepancies.** Fix small discrepancies on the branch directly (new commit, then re-check that item); list any remaining discrepancies explicitly on the PR so `technical-validation` does not proceed silently.
4. **Record and move status.** Reference the PR evidence comment from the parent Issue. This skill moves no status on its own; the transition is handled by `technical-validation`. End with the one-glance summary per `docs/board.md`.

## Outputs

- Screenshot evidence comment on the PR naming the SHA.

## Done criteria

- Every user-visible change checked against the spec's expected results on real-window shots (or `owner-test-only` recorded on the Issue).
- Evidence posted on the PR; discrepancies fixed (re-checked) or explicitly listed.
- Output referenced on the Issue and status handled per `docs/board.md`.
