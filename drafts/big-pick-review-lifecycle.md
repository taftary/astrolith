# big-pick-review-lifecycle

Authored by the AI agent at the owner's request, 2026-10-02. This is an agent review of the
workflow lifecycle: `docs/workflow.md`, `docs/board.md`, `docs/plans/workflow.md`,
`docs/validator-capabilities.md`, `AGENTS.md`, `.opencode/agents/*`, `.opencode/skills/*`,
`scripts/*`, `.github/workflows/ci.yml`, and the record of the last items (#35, #53, #62, #63).
Taking this draft follows the normal intake rule: the Issue body holds this file verbatim plus
this path, then the file is removed.

Scope: the lifecycle machinery only (agents, skills, docs, gates, board hygiene). The universe
product itself is untouched by this review.

## What I read

- `docs/workflow.md` (source of truth), `docs/board.md`, `docs/plans/workflow.md`,
  `docs/validator-capabilities.md`, `AGENTS.md`, `drafts/README.md`.
- `.opencode/agents/main.md`, `.opencode/agents/validator.md`, `.opencode/opencode.jsonc`.
- All 12 skills under `.opencode/skills/`.
- `.github/workflows/ci.yml`, `.github/PULL_REQUEST_TEMPLATE.md`, `.github/ISSUE_TEMPLATE/*`,
  `.gitignore`, `.agent/project-config.json`.
- Issue #63 end to end (spec, approval, preflight, runtime reports, validator marker, merge,
  owner verdict) and the #62 and #53 evidence, plus `cargo run -- --verify` conventions in
  `docs/universe/ladder.md` and `docs/universe/stack.md`.

## Verdict in one line

The lifecycle is well designed and mostly faithful on paper, but its strongest rules are prose,
and prose has already failed once in a way the owner cannot see: on #63 the runtime validator
reported BLOCKED with an unconfirmed requirements checklist, a `validator:pass` marker was posted
anyway, the PR merged, the item reached Done, and the Issue closed. Everything the owner relies on
to be true (the gate, the checklist, the board) depends on the main agent honouring its own
instructions. The requirements below turn those rules into scripts CI and the merge gate run.

## Findings, ranked

1. **A PASS was recorded over a BLOCKED report.** `.agent/validation/issue-63/20261001T210413Z/report.md`
   says `Verdict: BLOCKED`, checklist `UNCONFIRMED`, `C1` a placeholder, criteria `UNVERIFIABLE`.
   The Issue then carries `<!-- validator:pass sha=36622cb... -->`, PR #67 merged as `c482c40`,
   owner wrote `accept`, status Done, Issue closed. `.opencode/agents/validator.md` already says
   PASS requires every criterion MET and never converts BLOCKED into PASS. The rule was correct
   and unenforced.
2. **The requirements checklist was never built.** `.agent/validation/issue-63/requirements.md` is
   still `# Requirements (DRAFT - UNCONFIRMED)` with one template line `C1: (derive from body
   above)`. The validator's own contract is to build criteria from the owner's words first, so
   the layer that is supposed to compare the result to the notion never ran.
3. **The gates are instructions, not checks.** Merge gate, spec gate, correction bound, and the
   Done rule live in skill prose (`pull-request`, `technical-validation`, `implementation`,
   `correction`). `docs/plans/workflow.md` already flagged the CI-enforced validator-pass check as
   deferred and it is still deferred.
4. **"The exact commit that is merged" is undefined under squash.** `AGENTS.md` and
   `docs/workflow.md` require a pass for the merged commit; the gate compares the PR head SHA.
   On #63 those are different SHAs (`36622cb` validated, `c482c40` merged). The honest pairing is
   head SHA on the PR plus a runtime check of the squash commit on `main`.
5. **Post-merge validation has no owner.** The board lifecycle has `mergedPostMergePass`, the
   status table says "after merge, the merged result with post-merge validation awaits owner
   testing", and #53 carried a post-merge PASS comment. No skill step performs it, so the record
   exists only when someone remembers.
6. **Two loop bounds disagree.** `docs/workflow.md` (Correction) stops after two failed owner
   rounds and two consecutive validator fails. `technical-validation` says "Maximum 3 validator
   loops". Governance says the document wins and skills must not contradict it.
7. **The board's own rule is violated by every skill.** `docs/board.md`: "Never hardcode Status
   names or IDs in skills or scripts; read them from that file." Every skill passes
   `--status "In progress"`, `"Needs your answer"`, `"In review"` literally. A rename of an option
   would silently break all 12 skills while `project-config.json` sits unused for that purpose.
8. **Owner vocabulary is incomplete.** Approval words are defined (`approved`,
   `owner-test-only approved`, `abandoned`). Acceptance is not: #63 used `accept`. Nothing can
   check the Done rule, and the AI could in principle write the acceptance word itself.
9. **Approval and verdict provenance is unprotected.** Decision 3 in `docs/plans/workflow.md`: the
   AI and the owner share the `taftary` identity, so "the validator never writes code" and "only
   the owner writes `approved`" are rules, not facts. A dedicated identity is still on the deferred
   list.
10. **`docs/plans/workflow.md` is stale and cited as live.** It is the completed plan for #2 and
    still says Inbox, Working, Ready to test, and "eleven skills" (there are 12, and CI enforces
    12). `clarification` and most other skills still cite its skills table as a reference.
11. **`AGENTS.md` is behind the skill set.** It lists 11 skills and omits `wait-checks`; it never
    points at `docs/validator-capabilities.md`, the gates, or the plans policy.
12. **Session-driven intake has no resume path.** Nothing runs when no session is open, and there
    is no command that lists items waiting on the owner or whose last word was the owner's.
    `project.py` has `get`, `set-status`, `add`, `remove`, and no listing.
13. **Drafts "taken once" is remembered, not checked.** No command lists untaken drafts or tests
    whether an Issue already names a draft path.
14. **Sub-issues on the board.** `docs/board.md` records that the built-in auto-add sub-issues
    workflow is enabled and must be disabled in project settings. The `tasks` skill asks the agent
    to verify by hand. No script detects it.
15. **Intake commits bypass branch protection by design** (Decision 9, admin bypass) with no path
    guard on push to `main`.
16. **Visual validation of this product cannot be performed as written.** The app is a native
    window; `visual-validation` asks for a PR preview URL, which does not exist. The only proven
    validator capabilities are headless `--verify` and GitHub file reads
    (`docs/validator-capabilities.md`). Window changes have therefore been validated textually
    while the workflow claims a visual layer ran.
17. **Evidence is untracked and once unreadable.** `git status` shows untracked evidence under
    `.agent/validation/issue-62/` and `issue-63/`; one #63 evidence folder is listed by the
    filesystem but cannot be opened by path. The record of validation is not durable.
18. **CI does not enforce the documented pins.** `docs/universe/stack.md` requires zero clippy
    warnings and pins rustfmt, but CI runs plain `cargo clippy` and no `cargo fmt --check`. CI is
    ubuntu-only while the owner and validator run on Windows.

## Requirements

### A. Executable gates

- **R1** Add `scripts/gates/spec_gate.py --issue N`: exits 0 only when the Issue holds the owner's
  `approved` (or `owner-test-only approved`) comment, a spec comment containing goal, non-goals,
  acceptance criteria, test plan, testability needs, and a preflight pass comment (or the
  owner-test-only approval). Non-zero otherwise, printing exactly what is missing. The
  `implementation` skill calls it as step 1 and stops on non-zero, replacing the prose gate.
- **R2** Add `scripts/gates/merge_gate.py --issue N --pr P`: checks the `ci` job is green on the
  PR head SHA, an Issue comment holds `<!-- validator:pass sha=<full-head-sha> -->`, that comment
  carries `Runtime: PASS`, `Requirements: MET n/n` with `n` equal to the number of criteria, and
  `Drift: none`, the newest `.agent/validation/issue-N/<ts>/report.md` for that SHA has
  `Verdict: **PASS**`, and the PR body has no parent auto-close keyword. Non-zero means no merge.
  The `pull-request` skill calls it instead of listing the gate in prose.
- **R3** Add `scripts/gates/done_gate.py --issue N`: requires the owner's acceptance word, a
  post-merge validation record naming the merge commit on `main`, and no open `Needs correction`
  round, before the Issue may be closed or moved to Done.
- **R4** Add a CI job that fails any pull request whose body lacks a validator pass link, or whose
  linked pass SHA differs from the head SHA. This is the check deferred at
  `docs/plans/workflow.md` line 120.

### B. A pass can never overrule the runtime validator

- **R5** `requirements.md` per parent Issue must be confirmed before the validator gate: one
  criterion per acceptance criterion plus one per owner sentence that expresses an expectation,
  each citing its source (Issue body or comment line). The `UNCONFIRMED` marker is removed only by
  a comment on the Issue that names the SHA the checklist applies to. Confirmation is the owner's
  for anything ambiguous in the notion; the AI may confirm only mechanical derivations, and must
  say so.
- **R6** `scripts/validation/validate.py` must exit non-zero on `BLOCKED` and on an unconfirmed
  checklist, printing the blocking reason. `merge_gate.py` reads that exit status, so a BLOCKED
  round cannot be recorded as PASS. Today it writes the correct verdict into a report file and
  still exits 0.
- **R7** The PR's "Validator pass" section must be byte-identical to the Issue comment's verdict
  block, and the verdict comment must be the validator's own output pasted verbatim, not
  rewritten by the main agent. A script compares the two and fails on any difference.

### C. One machine-readable vocabulary

- **R8** Define the owner's words once in `docs/workflow.md` and mirror them in
  `.agent/project-config.json` (`approvalWords`: `approved`, `owner-test-only approved`,
  `abandoned`, and the new acceptance word). Add the acceptance word for Done (#63 used `accept`;
  pick one and use it everywhere).
- **R9** Add `project.py set-status --status-key <lifecycle key>` reading
  `.agent/project-config.json.lifecycle`, and change every skill to use the key instead of a
  literal status name, so `docs/board.md`'s no-hardcoding rule becomes true. A CI check fails if a
  skill or doc contains a literal status name or an option ID.
- **R10** Keep correction counters in markers, not prose: the `correction` skill posts
  `<!-- rounds: owner=<n> validator=<n> -->` and the bound check reads it. Align the validator
  loop bound on the document's 2 unless the owner decides otherwise.
- **R11** State the squash rule in `docs/workflow.md`: the gate covers the PR head SHA, and a
  runtime validation of the squash commit on `main` is required before Done. Both SHAs are
  recorded on the Issue.

### D. The post-merge step has an owner

- **R12** Add a post-merge stage, either as steps in `pull-request` and
  `present-for-owner-testing` or as a new skill: after squash merge, run
  `validate.py --issue N --sha <merge-sha>` on `main`, post the evidence, then move to In review.
  Until it runs, In review is an unsupported claim.

### E. Docs that cannot drift silently

- **R13** Archive `docs/plans/workflow.md` (it is the finished plan for #2) under
  `docs/plans/archive/` with a historical header, or re-header it as historical in place, and
  repoint every skill that cites it to `docs/workflow.md` and `docs/board.md`. It must stop
  advertising Inbox, Working, Ready to test, and "eleven skills".
- **R14** CI doc-drift checks: (a) no literal status name or option ID outside
  `.agent/project-config.json`; (b) every repository path referenced from a skill exists; (c) the
  13 stages in `docs/workflow.md` map to existing skill IDs; (d) relative Markdown links in `docs/`
  and `README.md` resolve.
- **R15** Update `AGENTS.md`: include `wait-checks`, point at `docs/validator-capabilities.md`, the
  gate scripts, and the rule that plans live in `docs/plans/`.

### F. Board hygiene and session ergonomics

- **R16** Confirm the built-in "Auto-add sub-issues to project" workflow is disabled, then add
  `project.py board-check`: fails when any `task`-labelled Issue is on the board. The `tasks`
  skill runs it instead of asking for a manual verification.
- **R17** Add `project.py resume`: parent Issues whose Status is `Needs your answer` or
  `In review`, with the last comment author, the last verdict, and the next stage to run. One
  command at session start, so nothing depends on the owner re-explaining where we stopped.
- **R18** Add `scripts/intake/list_drafts.py`: untaken drafts, and for each one whether an Issue
  already names its path, so "a draft is taken once" is enforced instead of remembered.
- **R19** Ask the owner whether to shorten the board option that the UI truncates as
  "Needs your ..." (option `8cca15ae`), e.g. "Answer". Renaming an option needs owner approval.

### G. Intake and branch protection

- **R20** Keep intake commits on `main` with admin bypass, and add a CI check on push to `main`:
  a commit that skipped a pull request may touch only `drafts/`, `docs/workflow.md`, and
  `.agent/`; anything else fails. The alternative, one auto-merging PR per draft, is the owner's
  call.

### H. A validator environment that matches this product

- **R21** Decide and document how a user-visible change to a native window is validated: either a
  capture mode in the app (for example `--capture <dir>` writing frames while replaying a scripted
  input sequence) proven once in `docs/validator-capabilities.md`, or an explicit per-item
  `owner-test-only` for window visuals recorded on the Issue. Today the workflow claims a visual
  layer while `visual-validation` can only be satisfied by noting "no UI change".
- **R22** Commit validation evidence with the branch, or attach it, instead of leaving it untracked
  in `.agent/validation/`: the report, the logs, and the requirements file for the validated SHA.
  Mirror the full report text on the Issue so the record survives a bad filesystem day.

### I. CI that matches the documented pins and the owner's platform

- **R23** Run `cargo clippy --locked --workspace -- -D warnings` and add `cargo fmt --check`, as
  `docs/universe/stack.md` already requires.
- **R24** Add a `windows-latest` job for `cargo check` and `cargo test`. The owner develops and the
  validator runs on Windows; today only Linux is proven.

## Decisions I need from the owner

1. Acceptance word for Done: `accepted` or `accept`? (R8)
2. Validator loop bound: 2 (document) or 3 (current skill text)? (R10)
3. Post-merge runtime validation: mandatory before every Done, or only for risky items? (R12)
4. Requirements checklist confirmation: owner confirms every item, or the AI derives it and the
   owner confirms only the ambiguous parts? (R5)
5. Intake commits: keep the admin bypass with the path guard, or one PR per draft? (R20)
6. Dedicated GitHub identity for the AI, deferred since #2: fund it now, or keep approval words and
   validator verdicts protected by rule only? (finding 9)
7. Board option rename for the truncated "Needs your ..." (R19).
8. Window visuals: build the capture mode, or accept `owner-test-only` for window visuals? (R21)

## Non-goals

- Changing the product, the scale ladder, or the frozen docs under `docs/universe/`.
- Changing the six board statuses or the thirteen-stage sequence.
- Event-driven automation.
- Relaxing any rule that protects the owner (notion preservation, validator gate, testability
  check, secrets handling). Those change only with owner approval recorded on an Issue or PR.

## Done when

- R1, R2, R3 and R6 exist and are called by the skills they replace in prose.
- The CI job from R4 fails a pull request with a stale or missing validator pass.
- No pull request can reach Done with a BLOCKED or unconfirmed requirements report.
- `docs/plans/workflow.md` is historical, `AGENTS.md` lists every skill, and the doc-drift checks
  from R14 are green.
- `project.py resume` and `scripts/intake/list_drafts.py` run at the start of a session.
- The eight decisions above are recorded on the Issue.
