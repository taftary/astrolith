# muse-review-lifecycle

Authored by the Muse Spark agent at the owner's request, 2026-10-02. This is an
agent review of the workflow lifecycle with focus on agents and docs:
`docs/workflow.md`, `docs/board.md`, `AGENTS.md`,
`.opencode/agents/main.md`, `.opencode/agents/validator.md`, the 12 skills
under `.opencode/skills/`, `docs/plans/workflow.md`, and
`docs/validator-capabilities.md`. Taking this draft follows the normal intake
rule: the Issue body holds this file verbatim plus this path, then the file is
removed.

Scope: the lifecycle machinery only, seen through its agents and its documents
(stage-to-skill mapping, agent contracts, status moves, vocabulary, doc
discipline). The universe product itself is untouched by this review.

Relationship to `drafts/big-pick-review-lifecycle.md`: that draft stands. Its
R1-R24 cover executable gates, CI enforcement, and board hygiene; this document
does not duplicate them. Where the two overlap (stale plan, AGENTS.md,
vocabulary, loop bound, status keys), this document endorses the big-pick
requirement and adds only the agent- and docs-side precisations. New
requirements here are numbered M1-M12.

## What I read

- `docs/workflow.md` (source of truth), `docs/board.md`, `AGENTS.md`,
  `drafts/README.md`, `drafts/big-pick-review-lifecycle.md`.
- `.opencode/agents/main.md`, `.opencode/agents/validator.md`,
  `.opencode/opencode.jsonc`.
- All 12 skills under `.opencode/skills/` (`clarification`,
  `specification`, `testability-preflight`, `planning`, `tasks`,
  `implementation`, `pull-request`, `technical-validation`,
  `visual-validation`, `correction`, `present-for-owner-testing`,
  `wait-checks`).
- `docs/plans/workflow.md`, `docs/validator-capabilities.md`,
  `.agent/project-config.json`, `.github/PULL_REQUEST_TEMPLATE.md`.
- Verification of cited lines with grep on 2026-10-02; no code was changed
  for this review.

## Verdict in one line

The agents are correctly scoped on paper and the documents say the right
things, but the stage-to-skill map is unwritten, five skills cite a stale plan
as a live reference, and the validator is denied the exact tools its duties
require — so the lifecycle currently runs on the main agent's good judgement
in the three places where judgement was supposed to be replaced by structure.

## Findings, ranked

1. **The 13-stage to 12-skill map exists nowhere.** `docs/workflow.md:90-104`
   defines 13 stages; `docs/workflow.md:226` lists 11 skills and omits
   `wait-checks` (same omission in `AGENTS.md:26`). In reality
   `clarification` covers stages 1-3, `pull-request` covers 9 and 11,
   `technical-validation` and `visual-validation` share stage 10,
   `present-for-owner-testing` covers 12-13, `correction` is labelled
   stage 13 but re-enters at 7, and `wait-checks` has no stage at all
   ("part of pull-request flow for #62"). The plan's "one skill per stage"
   (`docs/plans/workflow.md:82`) has never been true.
2. **Five skills cite the stale plan as a live skills table.**
   `clarification/SKILL.md:11`, `implementation/SKILL.md:8`,
   `technical-validation/SKILL.md:8`, `pull-request/SKILL.md:8`,
   `correction/SKILL.md:8` point at `docs/plans/workflow.md`, which still
   says Inbox, Working, Ready to test (`:60`, `:70`, `:79-80`, `:130-131`,
   `:182`) and "eleven skills" (`:41`, `:116`, `:221`) after the 2026-10-01
   rename to Todo, In progress, In review (recorded in `docs/board.md:9`
   and `.agent/project-config.json:33`).
3. **The validator cannot perform its written duties.** `validator.md:5-7`
   denies `edit`, yet `validator.md:73` orders it to write evidence under
   `.agent/validation/issue-<n>/<timestamp>/` and update `known-misses.md`.
   `validator.md:69-71` requires launching the real app and scanning browser
   console output, but the allowlist (`:18-58`) grants only
   `gh`/`curl`/`cargo test|run`/`python scripts/validation|sidebar` with no
   browser grant. `validator.md:67` mandates GitHub file reads at a commit
   SHA, yet `gh api` is `ask`, not `allow` (`:60-61`).
4. **Two skills break the every-skill-moves-status rule.**
   `docs/board.md:3` and the plan (`docs/plans/workflow.md:66`) require
   every skill to end with a board move. `wait-checks` has no status move
   and no `board.md` reference at all; `visual-validation/SKILL.md:20`
   says "No status change" while `:30` says status is "handled per
   `docs/board.md`".
5. **Two loop bounds disagree.** `technical-validation/SKILL.md:30` says
   "Maximum 3 validator loops"; `docs/workflow.md:174` and
   `correction/SKILL.md:18` say stop after two consecutive validator fails.
   Governance says the document wins.
6. **In review is defined twice and correction routing contradicts the
   board.** `docs/board.md:39` (PASS plus PR opened) and `:41` (merged
   plus post-merge pass) both lead to In review, but no skill performs the
   post-merge validation step. `correction/SKILL.md:22` keeps a validator
   FAIL in correction inside Needs correction, while `docs/board.md:38`,
   `docs/workflow.md:203`, and `project-config.json:24` route every
   validator FAIL to In progress. `docs/board.md:42` ("Back to Todo") has
   no source row in `docs/workflow.md:199-206`.
7. **The no-hardcoding rule is violated by the rulebook itself.**
   `docs/board.md:7` forbids hardcoded status names and IDs, yet every
   skill passes literal `--status "In progress"` / `"Needs your answer"` /
   `"In review"` strings, and `docs/board.md:9-18,57-73` prints the names
   and option IDs in prose. Endorses big-pick R9; the precise addition is
   M10 below.
8. **Acceptance vocabulary is undefined on the agent side.** Approval words
   are defined and denied to the AI; the acceptance word for Done is not
   (#63 used `accept`). Endorses big-pick R8; the precise addition is M9
   below.
9. **Only one skill tells the main agent how to brief the validator.**
   `technical-validation/SKILL.md:22-26` gives the exact three inputs for
   the gate; the preflight briefing lives only in `testability-preflight`.
   Nothing validates the brief, so a malformed brief degrades into a
   BLOCKED round instead of being rejected at launch.
10. **The capabilities registry is prose, not a contract.**
    `docs/validator-capabilities.md:3-6` logs past preflights as a table
    with no proving command per row, so preflight cannot check "only what
    is new" mechanically.
11. **`AGENTS.md` is behind the skill set and points nowhere.**
    It lists 11 skills (omits `wait-checks`), never points at
    `docs/validator-capabilities.md`, the gate scripts, or the
    `docs/plans/` policy. Endorses big-pick R15; the precise addition is
    M12 below.

## Requirements

### A. A written stage-to-skill map

- **M1** Rewrite the Skills section of `docs/workflow.md` as an explicit
  table of workflow stage(s) to skill ID, covering all 13 stages and all 12
  skills: `clarification` 1-3, `specification` 4, `testability-preflight`
  5, `planning` 6, `tasks` 7, `implementation` 8, `pull-request` 9 and 11,
  `technical-validation` plus `visual-validation` 10,
  `present-for-owner-testing` 12-13, `correction` 13-re-enters-7, and
  `wait-checks` marked as a utility sub-step of `pull-request` with no
  stage of its own. Mirror the table in `AGENTS.md`. Add a CI check that
  every stage maps to an existing skill ID and every skill directory maps
  to the table.

### B. The stale plan stops being citable

- **M2** Endorse big-pick R13 (archive or re-header
  `docs/plans/workflow.md`), with the exact repoint list: `clarification`
  `:11`, `implementation` `:8`, `technical-validation` `:8`,
  `pull-request` `:8`, `correction` `:8` must cite `docs/workflow.md` and
  `docs/board.md` instead. The archived copy keeps a historical header so
  the Inbox, Working, Ready to test, and "eleven skills" wording is never
  read as current again.

### C. A validator that can do its job

- **M3** Fix `.opencode/agents/validator.md` so duties and permissions
  match: grant write scoped to
  `.agent/validation/issue-<n>/<timestamp>/` and
  `.agent/validation/known-misses.md` while keeping the global `edit`
  deny; grant the browser/UI observation capability its runtime and
  requirements validation demand, or record the explicit fallback that UI
  evidence is `owner-test-only` per item; set read-only `gh api` to
  `allow` for the mandatory commit-SHA file reads; extend the runner
  allowlist to exactly what `scripts/validation/validate.py` invokes.
  Nothing else in the deny list changes.
- **M4** Add a brief contract both ways: every validator launch (gate and
  preflight) carries exactly the documented inputs and nothing else, and
  the validator rejects a malformed brief as BLOCKED naming the missing
  input before running anything. Put the gate checklist in
  `technical-validation` and the preflight checklist in
  `testability-preflight` so the two launch sites cannot drift.

### D. Status moves without contradictions

- **M5** Declare the only two exceptions to the every-skill-moves rule in
  `docs/board.md`: `wait-checks` (utility, moves nothing, caller moves)
  and `visual-validation` (moves nothing, `technical-validation` moves).
  Fix the `visual-validation` self-contradiction (`:20` vs `:30`) to say
  exactly that.
- **M6** Single loop bound of 2 everywhere: change
  `technical-validation/SKILL.md:30` from "Maximum 3 validator loops" to
  the document's two consecutive validator fails (endorses big-pick R10).
  A bound of 3 returns only by owner decision recorded on an Issue.
- **M7** Give In review one definition with two recorded arrivals: PASS
  plus PR open (proposal under test on the branch), and merged plus
  post-merge validation (merged result awaiting owner testing). Name the
  post-merge step's owner (endorses big-pick R12) and either source
  `docs/board.md:42` ("Back to Todo") in `docs/workflow.md` or delete the
  row.
- **M8** Align `correction/SKILL.md:22` with `docs/board.md:38`: a
  validator FAIL anywhere, including inside a correction round, routes to
  In progress. Needs correction is entered only on the owner's test
  verdict, never on a validator verdict.

### E. Vocabulary and keys the agents actually read

- **M9** Endorse big-pick R8 for the word list including the new
  acceptance word, and add the agent side: the words live once in
  `docs/workflow.md` and are mirrored in `.agent/project-config.json`;
  `main.md` keeps the never-emit rule extended to the acceptance word;
  skills match owner comments against the config, never against literals
  in prose.
- **M10** Endorse big-pick R9 (`--status-key` reading the `lifecycle`
  map), and add: the `lifecycle` keys (`pickedUp`, `branchCreated`,
  `validatorFail`, `validatorPassPrOpen`, `blocked`, `mergedPostMergePass`,
  `paused`) are the only vocabulary skills may use; `docs/board.md` keeps
  its ID table as documentary reference but its command recipes use keys.

### F. Docs that stay true

- **M11** Make the capabilities registry a contract: every row of
  `docs/validator-capabilities.md` carries the capability, the exact
  proving command, the Issue, and the date. `testability-preflight`
  checks only needs not already proven by a row.
- **M12** State the plans policy in `AGENTS.md`: active plans live in
  `docs/plans/<slug>.md`, finished plans get a historical header, and
  `AGENTS.md` lists all 12 skills and points at
  `docs/validator-capabilities.md`, the gate scripts, and this policy
  (extends big-pick R15).

## Decisions I need from the owner

1. Loop bound: 2 (document) or 3 (current skill text)? Shared with
   big-pick Q2; one answer covers both (M6).
2. Acceptance word for Done: `accepted` or `accept`? Shared with big-pick
   Q1; one answer covers both (M9).
3. Validator evidence writes (M3): grant the validator scoped write to its
   evidence paths, or keep the full deny and have the main agent place the
   validator's pasted output?
4. `wait-checks` and `visual-validation` (M5): documented no-move
   exceptions as proposed, or give them explicit status moves?
5. `docs/board.md:42` "Back to Todo" (M7): source it in `docs/workflow.md`
   or delete the row?

## Non-goals

- Changing the product, the scale ladder, or the frozen docs under
  `docs/universe/`.
- Changing the six board statuses or the thirteen-stage sequence.
- Event-driven automation.
- Relaxing any rule that protects the owner (notion preservation,
  validator gate, testability check, secrets handling). Those change only
  with owner approval recorded on an Issue or PR.
- Duplicating big-pick R1-R24 on gates, CI jobs, intake guards, and board
  tooling. On any overlap, that draft wins and this one defers.

## Done when

- The M1 table exists in `docs/workflow.md`, is mirrored in `AGENTS.md`,
  and the stage-to-skill CI check is green.
- The validator's duties and permissions match (M3) and both launch sites
  enforce the brief contract (M4).
- `wait-checks` and `visual-validation` are the only no-move skills and
  say so (M5); the bound is 2 everywhere (M6); In review has one
  definition with two recorded arrivals (M7); validator FAIL always routes
  to In progress (M8).
- Words and status keys are read from `.agent/project-config.json`, never
  from literals (M9, M10).
- `docs/plans/workflow.md` is historical, the five skills repoint at the
  live documents (M2), and every registry row carries its proving command
  (M11, M12).
- The five decisions above are recorded on the Issue.
