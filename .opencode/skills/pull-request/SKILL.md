---
name: pull-request
description: Load when opening or merging a pull request for a parent Issue (workflow stages 9 and 11). Use to create the PR from the template and to run the merge gate.
---

# Pull Request

One feature branch and one pull request per round of work on a parent Issue, per `docs/workflow.md` (Branching and Merge, Project Board) and the stage-to-skill table in `docs/workflow.md` (Skills).

## Inputs

- Feature branch (`feat/<N>-<slug>` or `fix/<N>-<slug>`).
- Parent Issue number `#N`.
- Sub-issue numbers for the task list.
- PR template at `.github/PULL_REQUEST_TEMPLATE.md`.

## Steps

1. **Create PR from the template.** Body contains:
   - `Related to #N` (manual link).
   - Sub-issue task list (e.g. `- [ ] #A`, `- [ ] #B`).
   - Validator pass fields (filled after `technical-validation`).
   - NO auto-close keyword (`Closes`, `Fixes`, `Resolves` + `#N`) for the parent Issue. Only sub-issues may use auto-close keywords. CI parent auto-close guard enforces this.
2. **Wait for technical validation + validator gate** (see `technical-validation` skill). Do not merge before both pass.
3. **Merge gate (run the script, do not hand-check).** `python scripts/gates/merge_gate.py --issue N --pr P` must exit 0: CI green on the head SHA, verbatim validator PASS for the full head SHA on the Issue, newest report verdict PASS, no parent auto-close keyword. Then `python scripts/gates/compare_verdict.py --issue N --pr P` must report IDENTICAL (the PR's Validator-pass section is byte-identical to the Issue verdict block, pasted verbatim from the validator's output). Non-zero means no merge.
   - CI wait uses the `wait-checks` skill (`python scripts/sidebar/checks.py wait --pr <N> --timeout 600 --interval 15`; progress to stderr, JSON to stdout). Run it in the background for long waits.
   - PR body keyword check: always via file input to avoid PowerShell quoting breakage: save the body to a file, then `python scripts/sidebar/development.py check-body --body-file <path>` (never pass a quoted body inline with `--body "..."`).
   - Fill the Validator pass fields only after the validator passes on the final head (no more commits after). PR-body edits do not retrigger CI, so close and reopen the PR once after filling them to get a fresh CI evaluation of the final body.
4. **Merge.** Squash merge the PR, then delete the branch.
5. **Post-merge validation (mandatory, owns the In review move).** On `main` at the merge commit, run `python scripts/validation/validate.py --issue N --sha <merge-sha>`, post the evidence on the Issue naming the merge SHA, then set `--status-key mergedPostMergePass`. Until this runs, In review is an unsupported claim: never move there on PR open.
6. **Close sub-issues.** Close every sub-issue in the PR task list after the squash merge (one `gh issue close` each, comment naming the PR and merge SHA).
7. **Hand off.** A merged PR moves the item to In review, never to Done. Never close the parent Issue here. Hand off to the `present-for-owner-testing` skill.
8. **Record and move status.** Post the merge record (PR number, head SHA, merge SHA, validator pass link) on the parent Issue. Status was moved in step 5 (verified by re-read). End with the one-glance summary per `docs/board.md`.

## Outputs

- Merged PR (squash), branch deleted.
- Merge record comment on the parent Issue naming the head SHA.

## Done criteria

- PR was created from the template with `Related to #N` and no parent auto-close keyword.
- Merge gate passed (`merge_gate.py` exit 0, `compare_verdict.py` IDENTICAL).
- PR squash-merged, post-merge validation recorded for the merge SHA, all listed sub-issues closed, branch deleted, status moved to In review per `docs/board.md`, output recorded on the Issue.
