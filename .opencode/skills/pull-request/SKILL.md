---
name: pull-request
description: Load when opening or merging a pull request for a parent Issue (workflow stages 9 and 11). Use to create the PR from the template and to run the merge gate.
---

# Pull Request

One feature branch and one pull request per round of work on a parent Issue, per `docs/workflow.md` (Branching and Merge, Project Board) and `docs/plans/workflow.md` skills table.

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
3. **Merge gate (all must hold):**
   - CI job `ci` is green on the head commit — wait with the `wait-checks` skill (`python scripts/sidebar/checks.py wait --pr <N> --timeout 600 --interval 15`; progress to stderr, JSON to stdout). Run it in the background for long waits.
   - An Issue comment holds `<!-- validator:pass sha=<full-sha> -->` where `<full-sha>` equals the PR head SHA exactly. A pass applies only to that commit; any new commit needs a new validator pass.
   - PR body keyword check: no auto-close keyword targets the parent Issue. Always check via file input to avoid PowerShell quoting breakage: save the body to a file, then `python scripts/sidebar/development.py check-body --body-file <path>` (never pass a quoted body inline with `--body "..."`).
4. **Merge.** Squash merge the PR, then delete the branch.
5. **Hand off.** A merged PR moves the item to In review, never to Done. Never close the parent Issue here. Hand off to the `present-for-owner-testing` skill.
6. **Record and move status.** Post the merge result (PR number, head SHA, validator pass link) on the parent Issue. Set Status to In review via `python scripts/sidebar/project.py set-status` (merged → In review; verified by re-read). End with the one-glance summary: Project/Status, Milestone, Relationships, Linked branches/PRs, Subscription, Last validation verdict.

## Outputs

- Merged PR (squash), branch deleted.
- Merge record comment on the parent Issue naming the head SHA.

## Done criteria

- PR was created from the template with `Related to #N` and no parent auto-close keyword.
- Merge gate passed (CI green, `validator:pass` marker matches head SHA, keyword check clean).
- PR squash-merged, branch deleted, status moved to In review per `docs/board.md`, output recorded on the Issue.
