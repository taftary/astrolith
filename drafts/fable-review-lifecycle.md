# Fable review: workflow lifecycle (agents and docs)

Review of the lifecycle as written in `docs/workflow.md`, `docs/board.md`, `docs/plans/workflow.md`, `AGENTS.md`, `CLAUDE.md`, `.opencode/agents/*.md`, the twelve skills under `.opencode/skills/`, `.github/`, `.agent/project-config.json`, and the scripts the skills call (`scripts/validation/validate.py`, `scripts/sidebar/*.py`). Written by the Fable agent on 2026-10-02 without reading the other review drafts in `drafts/`, so overlaps with them are unintentional and the owner can merge or dedupe when taking the drafts.

The lifecycle is in good shape: the thirteen stages are all covered by a skill, every rule in `docs/workflow.md` has a home, status moves go through one verified script, and the validator is a real subagent with a deny-by-default permission list. What follows are the places where the documents disagree with each other, promise something no procedure delivers, or push technical work onto the owner. Each finding ends with a requirement (F1 to F22). Requirements are grouped by what they protect.

## A. Documents that contradict each other

Per `docs/workflow.md` (Sync and Governance) the workflow document wins. Every item here is a case where a skill, the board reference, or the config says something else.

### F1. Validator loop bound: two versus three

`docs/workflow.md` (Correction): "after two consecutive validator fails on the same item, the AI stops". `.opencode/skills/technical-validation/SKILL.md` step 4: "Maximum 3 validator loops; after that set Status to Needs your answer".

Requirement: the technical-validation skill says two, matching the workflow document and the correction skill. The number is written once, in `docs/workflow.md`, and skills refer to it as "the bound in workflow.md (Correction)" rather than repeating a digit.

### F2. Where a validator FAIL sends an item during a correction round

`docs/board.md` event table: "Validator verdict = FAIL (any attempt) -> In progress". `.opencode/skills/correction/SKILL.md` step 6: "A validator FAIL during the round keeps the item in Needs correction". `docs/workflow.md` Status Transitions supports the correction skill ("Needs correction ... stays here while the AI corrects, re-validates, and merges") but its In progress row says "a validator FAIL returns the item here" without excluding correction rounds.

Requirement: `docs/workflow.md` Status Transitions states the rule explicitly (a FAIL on a correction round stays in Needs correction; a FAIL on a first round goes to In progress), and `docs/board.md` plus `.agent/project-config.json` (`lifecycle.validatorFail`) encode both cases instead of one.

### F3. Abandonment: Done versus Todo

`docs/workflow.md`: abandonment moves the item to Done with reason `abandoned`. `docs/board.md` event table: "Work paused or abandoned -> Back to Todo", and `.agent/project-config.json` has `lifecycle.paused: "Todo"`. The workflow document's Status Transitions table has no transition that re-enters Todo at all.

Requirement: remove "abandoned" from the board's "Back to Todo" row. If "paused" is a real state the owner wants, add it to `docs/workflow.md` Status Transitions first (what enters it, what leaves it); otherwise drop the row and the config key.

### F4. The twelfth skill is not in the source of truth

`docs/workflow.md` (Skills) lists eleven skills "one per stage". `docs/plans/workflow.md` says "the document's one skill per stage holds" and names eleven. `.github/workflows/ci.yml` requires exactly twelve, including `wait-checks`, which was added in PR #72 without touching `docs/workflow.md`. `AGENTS.md` (References) also omits it. This is the governance rule ("this document is changed first ... same pull request") being broken by the process's own tooling PR.

Requirement: `docs/workflow.md` Skills section names `wait-checks` as a helper skill (not a stage), `AGENTS.md` lists it, and CI's "Workflow files check" additionally fails when a skill ID present in `.opencode/skills/` is not named in `docs/workflow.md`. The `wait-checks` description drops "for #62": a skill description is a procedure, not a changelog.

### F5. Skills read a stale plan as a live reference

`.opencode/skills/clarification/SKILL.md`, `implementation`, `pull-request`, `technical-validation`, and `correction` cite "the Skills table in `docs/plans/workflow.md`" as an input. That plan still says the Status options are "Inbox, Working, Ready to test", that there are eleven skills, and that `drafts/notion.md` is the first draft to take. It is a historical artifact being used as a current reference.

Requirement: move the skills table (ID, stage, inputs, outputs, done-when) into `docs/workflow.md` or `docs/board.md`, re-point every skill at that location, and mark `docs/plans/workflow.md` as historical in its first line (date it was completed, note that it is not maintained).

### F6. Two instruction surfaces disagree

`CLAUDE.md` says "Do not touch README.md, workflow files, skills, agents, or board docs." `docs/workflow.md` (Sync and Governance) says "The AI may propose changes to skills freely." `CLAUDE.md` also describes `universe-app --verify` as a "headless stub (exits 0)", which is no longer true (it replays the journey and prints PASS per level). `CLAUDE.md` is a root file read by every Claude-based agent, including the main agent that must edit skills.

Requirement: `CLAUDE.md` is scoped by its own first line to the `universe-*` crates and should say so in a way the agent cannot miss, or the "do not touch" line is rewritten as "changes to workflow files, skills, agents, and board docs follow `docs/workflow.md` (Sync and Governance), not this file". Its stale description of `--verify` is corrected.

### F7. Skill paths are written two ways

Five skills use `../../../docs/workflow.md` (relative to the skill file); the others use `docs/workflow.md` (relative to the repository root, which is where the agent actually runs commands). The relative-to-skill form is misleading once the skill text is loaded into a session whose working directory is the repository root.

Requirement: all skills use repository-root-relative paths.

## B. Promises with no procedure behind them

### F8. Post-merge validation is required by two documents and performed by none

`docs/board.md`: "PR merged AND post-merge validation passes (owner test still pending) -> In review". `docs/workflow.md` In review row: "after merge, the merged result with post-merge validation awaits owner testing". No skill runs anything after the merge: `pull-request` step 6 moves to In review immediately, and `present-for-owner-testing` posts the testing comment. The validator pass covers the branch head SHA; a squash merge creates a different commit on `main` that nobody has validated.

Requirement: define post-merge validation in `docs/workflow.md` (what runs, on which commit, who records it) and give it a home: either a step in `pull-request` after the squash merge (run `scripts/validation/validate.py --sha <merge-sha>` on `main`, post the result naming the merge SHA, then move to In review) or an explicit statement that CI on the push to `main` is the post-merge validation and the In review move waits for it to be green (the `wait-checks` skill can wait on the merge SHA).

### F9. `requirements.md` has no owner and makes the owner do technical work

`docs/workflow.md` (Validator Environment) and `.opencode/agents/validator.md` require the validator to check `.agent/validation/issue-<n>/requirements.md`. `scripts/validation/validate.py` drafts that file if missing, marks it UNCONFIRMED, and returns BLOCKED until "the owner confirms this file", and a criterion only becomes MET when it carries a `(verify: contains TEXT in LOG)` probe. Nothing in any skill says who writes the criteria or the probes, and asking the owner to confirm a probe-annotated checklist contradicts Owner Needs ("does not need to ... perform technical validation ... inspect implementation details").

Requirement: the `specification` skill produces `requirements.md` from the approved spec (one criterion per acceptance criterion, in the owner's words, with the AI adding the probe), and owner approval of the spec counts as confirmation of the criteria text. The "owner must confirm" wording in `validate.py` and the drafted file is replaced by "derived from the approved spec on `<date>`; approval comment: `<link>`". The owner never sees the probe syntax.

### F10. The runtime validator is generic in the docs and product-specific in code

`docs/workflow.md` and `validator.md` promise "real env/migrations/seed data", "HTTP 200 / port open", "browser console, container logs". `scripts/validation/validate.py` runs exactly `cargo test --workspace` and `cargo run -p universe-app -- --verify` twice. Three further gaps: it runs `cargo test` without `--locked` although CI and `CLAUDE.md` require `--locked`; `--sha` is only printed in the report and never compared with the worktree, so a verdict can name a commit that was not the one tested; and the validator cannot run `git`, so it has no way to check what it is testing.

Requirement: the docs describe what the tool does today (headless `--verify`, repeat-run determinism, log scan) and name the web-style checks as "when the product has them". `validate.py` uses `--locked`, and when `--sha` is given it records `git rev-parse HEAD` and `git status --porcelain` and fails with BLOCKED when HEAD differs from `--sha` or the tree is dirty. `validator.md` allows the read-only `git rev-parse *`, `git status *`, and `git log *` commands so the validator can do the same.

### F11. Visual validation assumes a web preview

`.opencode/skills/visual-validation/SKILL.md` requires a "PR preview URL". This product is a desktop Bevy window: there is no preview deployment and no screenshot path. The skill is therefore unrunnable for every UI change the project has shipped so far, and `technical-validation` can only "note that explicitly".

Requirement: the skill gets a desktop path (how to launch the window at the head SHA, how to capture a screenshot or a `--verify` snapshot, where evidence goes), or `docs/workflow.md` states that for this product the `--verify` PASS lines are the visual evidence and the skill says so.

### F12. Sub-issue closure has no rule

The PR template lists sub-issues as a `- [ ] #N` task list (no auto-close), `implementation` says sub-issue closes "may" use keywords, and the history shows sub-issues closed by hand at merge time (for #62, all four closed within two seconds of the merge). Nothing says who closes them or when.

Requirement: `pull-request` step 4 closes every sub-issue in the task list after the squash merge (one `gh issue close` each, comment naming the PR and merge SHA), and the done criteria of that skill include "all listed sub-issues closed".

## C. Validator independence

### F13. "Fresh context, three inputs" is not what the validator receives

`docs/workflow.md`: the validator "does not receive the main agent's reasoning or transcript". In practice `validator.md` allows `gh issue view *` (the whole Issue, including the plan, implementation summary and every main-agent comment), `gh pr diff *` (the code, which the workflow says the validator must not use to perform the behavior under test), and `validate.py` itself fetches the Issue body and all comments to draft `requirements.md`.

Requirement: `validator.md` drops `gh pr diff *` from the allowlist. `docs/workflow.md` states honestly that the validator may read the Issue for the verbatim notion and approved spec only, and the validator prompt in `technical-validation` step 3 tells it which two comments those are (by link) so it does not have to read the rest.

### F14. The validator tests in the main agent's worktree

The validator is launched as a subagent in the same repository checkout the main agent just edited. It has no `git` and cannot check out the PR head. If the main agent has uncommitted changes, the validator tests those, and the verdict names a SHA that may not match.

Requirement: `technical-validation` step 3 requires a clean tree at the PR head before launching the validator (`git status --porcelain` empty, `git rev-parse HEAD` equals the PR head SHA) and passes both values in the validator's inputs; the validator's first action is to confirm them (see F10). A worktree under `tmp/agent-shared/` per round is acceptable as a stronger alternative and should be named as such in the skill.

### F15. Validation evidence changes the SHA it validates

Evidence lands under `.agent/validation/issue-<n>/<timestamp>/`. That directory is partly committed and partly untracked (`git status` shows `issue-62/` and `issue-63/` untracked while `issue-35/` and `issue-53/` are tracked), and `.gitignore` says nothing about it. If the main agent commits evidence to the feature branch, the head SHA changes and the validator pass for the previous SHA is void.

Requirement: decide once in `docs/workflow.md`: evidence is either never committed (`.agent/validation/**/<timestamp>/` in `.gitignore`, the Issue comment is the record) or committed to `main` only in a separate bookkeeping commit after merge. `requirements.md` (F9) is committed with the feature PR because it is part of the specification.

## D. Rules that need a precise form

### F16. Approval detection

Skills "look for the exact comment `approved`", but the AI's own comments routinely contain the word ("spec v4 approved" appears in `CLAUDE.md`). With the owner and AI sharing one GitHub identity, nothing distinguishes the owner's approval from a summary that mentions approval.

Requirement: `docs/workflow.md` defines an approval as a comment whose body, trimmed, is exactly one of the three words (or begins with it on its own line), and the specification skill records the comment URL on the Issue. `AGENTS.md` is tightened from "never posts the approval words as a comment" to "never posts a comment whose body begins with an approval word". Until a bot identity exists, this is the whole protection and should be stated as such.

### F17. Round counters as markers

The correction skill reads the Issue prose to find the current round and fail counts. The validator already uses machine-readable markers (`<!-- validator:pass sha=... -->`).

Requirement: the correction skill posts `<!-- correction:round k owner-fails=m validator-fails=v -->` with each round, and reads the latest marker instead of prose.

### F18. Specification revisions

Issue #35 reached "spec v4". No skill says how a spec is revised after approval, what happens to the earlier approval, or how the validator knows which version is the frozen one.

Requirement: the specification skill numbers each spec comment (`Spec v<k>`), states that a new version voids the previous approval and requires a new `approved`, and the validator's input names the approved version by link.

### F19. The pre-merge "In review" window is the AI's turn

`docs/workflow.md` says In review is the owner's turn, yet the item enters In review when the validator passes and the PR is still open, before merge. The owner cannot test an unmerged PR (owner tests the merged result), so during that window the status says "owner" while the next action (merge) belongs to the AI.

Requirement: either the merge happens in the same skill run as the validator pass so the window is momentary and the document says so, or In review is entered only on merge and the "validator PASS and PR opened -> In review" row is removed from `docs/board.md`, `technical-validation` step 5, and `.agent/project-config.json` (`validatorPassPrOpen`).

## E. Intake and branches

### F20. Agent-authored drafts and PRs without a parent Issue

`drafts/README.md` and `docs/workflow.md` say drafts are "written in the owner's own words". This file, and the two review drafts before it, were written by agents and landed through PRs on branches named `feat/<pr-number>-...` because the CI branch guard demands an Issue number and no Issue exists. The workflow has no path for repository-maintenance changes that are not a notion (review drafts, tooling fixes, doc corrections).

Requirement: `docs/workflow.md` (Drafts Folder) allows agent-authored drafts when the owner asks for them in a session, with the author named in the file's first paragraph. Branching and Merge defines a `chore/<slug>` branch form for maintenance PRs with no parent Issue, the CI branch guard accepts it, and the PR body must state "no parent Issue" so the auto-close guard and merge gate know no validator pass is expected.

### F21. The board still auto-adds sub-issues

`docs/board.md` notes that GitHub's built-in "Auto-add sub-issues to project" workflow is enabled and "must be disabled in project settings; until then, flag them in the backfill". That is a known configuration defect carried as a footnote.

Requirement: it becomes a Todo Issue (type `bug`) so it is visible on the board and gets fixed, and the footnote in `docs/board.md` is removed when it is.

### F22. The one-glance summary is inconsistent

Six skills end with the one-glance summary (Project/Status, Milestone, Relationships, Linked branches/PRs, Subscription, Last validation verdict); `specification`, `testability-preflight`, `planning`, `tasks`, `visual-validation`, and `present-for-owner-testing` do not.

Requirement: the summary is defined once in `docs/board.md` and every skill's final step says "end with the one-glance summary per `docs/board.md`".

## What I would do first

1. F1, F2, F3, F7, F22: pure document fixes, one PR, no product change.
2. F4, F5, F6: bring the source of truth back in line and stop skills reading the stale plan.
3. F9, F10, F13, F14: make the validator verdict mean what it says (right SHA, clean tree, criteria from the approved spec, no code reading).
4. F8, F12, F15, F19: close the gaps around merge.
5. F11, F16, F17, F18, F20, F21: the rest.

Items in groups 1 and 2 touch only skills and docs. Items that change the instructions in `AGENTS.md` or the rules that protect the owner (F13, F16, F19) need the owner's explicit approval per Sync and Governance before they are implemented.
