---
name: visual-validation
description: Load when a pull request contains any user-visible UI change (part of workflow stage 10). Use to check rendered UI against the spec on the PR preview.
---

# Visual Validation

Required for any user-visible UI change, optional otherwise. Runs inside `technical-validation` before the validator gate, per `docs/workflow.md` (Validation Layers). This product is a native window: there is no PR preview URL. Evidence is the `frame_proof.py` fragment (`scripts/validation/frame_proof.py`, run inside `validate.py`): after frames captured at the head commit, before frames captured at the base commit in a detached worktree, converted to pictures and published to the `validation-evidence` release. When the change cannot be captured, record per-item `owner-test-only` on the Issue and skip the rest.

## Inputs

- Head commit SHA plus `--capture <dir>` output (frames + snapshot log), or the `owner-test-only` record link.
- Approved spec: test plan steps + expected results for every user-visible change.

## Steps

1. **Check each change.** For every user-visible change, perform the spec's human steps against the capture output and compare what is seen against the spec's expected results.
2. **Build the proof fragment.** The pictures live in the verdict comment on the parent Issue, not on the PR: paste the `frame-proof.md` fragment into the verdict comment. Layout rule: the fragment starts with a blank line, and a blank line must separate the verdict marker line from the first picture (a line opening with `<!--` is a CommonMark HTML block and swallows any image on that line). The PR keeps only the text summary. Name the commit SHA checked.
3. **Handle discrepancies.** Fix small discrepancies on the branch directly (new commit, then re-check that item); list any remaining discrepancies explicitly on the PR so `technical-validation` does not proceed silently.
4. **Record and move status.** Reference the PR evidence comment from the parent Issue. This skill moves no status on its own; the transition is handled by `technical-validation`. End with the one-glance summary per `docs/board.md`.

## Outputs

- Proof fragment (`frame-proof.md`) naming the SHA, pasted into the verdict comment on the parent Issue; text summary on the PR.

## Done criteria

- Every user-visible change checked against the spec's expected results via the frame proof (or `owner-test-only` recorded on the Issue).
- Fragment pasted into the verdict comment with the layout rule kept (blank line after the marker); discrepancies fixed (re-checked) or explicitly listed.
- Output referenced on the Issue and status handled per `docs/board.md`.
