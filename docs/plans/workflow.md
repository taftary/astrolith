# Implementation Plan: AI-Driven GitHub Workflow

> HISTORICAL — completed plan for Issue #2 (2026-10-01). Not maintained. Do not use as a live reference; the source of truth is `docs/workflow.md` and the reference is `docs/board.md`. Status names below (Inbox, Working, Ready to test) and the "eleven skills" count predate the 2026-10-01 rename to Todo, In progress, In review and the `wait-checks` skill.

Source: `drafts/notion.md` (the notion and its specification; it becomes `docs/workflow.md` when taken, see Decision 4).
Location: `docs/plans/workflow.md`. Plans are workflow artifacts and never live in `drafts/`, which is the owner's intake folder.
Owner of this plan: the AI. Per the workflow document, the plan does not need owner approval; the decisions below proceed unless the owner objects.
Next stage: Tasks. First act: take `drafts/notion.md` (parent Issue `#N` with the content verbatim; the intake commit moves the file to `docs/workflow.md`). Then one sub-issue per row of the work breakdown.

## Goal

Build the workflow described in `drafts/notion.md` inside this repository and its GitHub project, so that the first real notion can travel from Issue to Done exactly as the document says, with every rule in the document having a concrete home (instruction, skill, agent, CI check, or GitHub setting).

## Non-goals

- Building any product feature. The repository has no product yet; this plan builds the process only.
- A validator environment for a real product (preview deployments, test accounts, seed data). The workflow document already says missing capability is built as its own work item when the first product notion needs it.
- Event-driven automation (a bot that wakes up on every GitHub event). Version 1 is session-driven; see Decision 2.
- A production release process. The document places it after Done.

## Decisions and Assumptions

1. **Harness is OpenCode V2.** The document's vocabulary maps one-to-one: instructions = `AGENTS.md`, skills = `.opencode/skills/<id>/SKILL.md`, agents = `.opencode/agents/<id>.md`. A subagent runs in a child session with fresh context, which is exactly the validator's "fresh context, three inputs" requirement.
2. **Session-driven in v1.** The owner opens an OpenCode session; the AI lists untaken drafts in `drafts/` and asks which to take, and the owner can also state a notion directly or ask the AI to process the Inbox (Issues filed on GitHub). Nothing runs while no session is open. An event-driven trigger is a later item. *This is the one decision that changes how the owner interacts; the owner should confirm or redirect it.*
3. **GitHub access is the owner's `gh` login** (`taftary`, scopes `repo`, `project`, `workflow`, already present). Consequence: the AI and the owner share one identity, so the explicit approval words (`approved`, `owner-test-only approved`, `abandoned`) are protected by rule, not by account. Rule added to the instructions: the AI never writes those words as a comment. A dedicated bot identity is a later hardening item.
4. **`drafts/notion.md` is the first draft to be taken.** `drafts/` is the owner's intake folder: a draft becomes an Issue and the file is removed. This draft is the special case named in the workflow document, because its content is also the process document, so the take moves it to `docs/workflow.md` instead of deleting it; the Issue body still holds it verbatim. Internal references are updated in the same change.
5. **Specification and plan live as Issue comments in v1** (the document allows comments or linked files). Files are used only when a comment would be unwieldy.
6. **Board auto-add uses a positive label filter** (`is:issue is:open label:notion,feedback,bug`). Sub-issues carry only the `task` label, so they never reach the board. This avoids relying on negative filters.
7. **Validator evidence is text in v1** (steps taken, observed text, status codes, commit SHA). Screenshot hosting is part of the deferred validator-environment item.
8. **The dry run uses a documentation notion** ("the README explains what Astrolith is and links to the workflow document"), because its public interface, the rendered file on GitHub at a commit SHA, exists without a product. The owner writes it as a draft file so the dry run also exercises intake.
9. **Intake commits go directly to `main`.** Removing or moving a draft is bookkeeping: it touches only `drafts/` (plus the one-time move to `docs/workflow.md`), and the Issue it names is the record. These commits land under the owner's admin bypass; everything else goes through a pull request. The alternative is a tiny auto-merged PR per draft, if the owner prefers protection with no exceptions.
10. **Agent temp shared workspace lives outside `drafts/`.** `drafts/` stays pure intake (Option A). Agents create temp shared files only under gitignored `tmp/agent-shared/`; anything obsolete there is deleted directly by the main agent and never committed.

## Deliverables

When the plan is complete, the repository contains:

```text
AGENTS.md                               instructions (the seven rules + approval-words rule + pointers)
.opencode/opencode.jsonc                default_agent: main; shared permissions
.opencode/agents/main.md                primary agent
.opencode/agents/validator.md           subagent, read-only, narrow shell allowlist
.opencode/skills/<11 skills>/SKILL.md   one per stage (table below)
docs/workflow.md                        the workflow document (moved from drafts/notion.md when taken)
docs/plans/<slug>.md                    plans too long for an Issue comment (this file is the first)
docs/board.md                           project number, field and option IDs, gh commands for every status move
docs/validator-capabilities.md          registry of proven validator capabilities
tmp/agent-shared/                       gitignored agent temp shared files, deleted directly when obsolete (never committed)
.gitignore                              ignores tmp/agent-shared/
drafts/README.md                        the intake convention; the only file that stays in drafts/
.github/ISSUE_TEMPLATE/notion.yml       one free-text field, applies label notion
.github/ISSUE_TEMPLATE/feedback.yml     same, label feedback
.github/ISSUE_TEMPLATE/bug.yml          same, label bug
.github/PULL_REQUEST_TEMPLATE.md        Related to #N, validator pass link + SHA, no auto-close for parent
.github/workflows/ci.yml                checks listed under CI
README.md                               what Astrolith is, link to docs/workflow.md
```

And the GitHub side has:

- Labels: `notion`, `feedback`, `bug`, `task`, `owner-test-only`, `abandoned`. That is the whole set.
- One Project "Astrolith" under `taftary`, linked to the repo, Status options exactly: Inbox, Needs your answer, Working, Ready to test, Needs correction, Done.
- Auto-add workflow with the filter from Decision 6.
- Branch protection on `main`: pull request required, zero approvals required (the owner never reviews code), `ci` required to pass, no force push.

## Skills

Each skill is loaded only at its stage. Every skill ends by moving the board status per `docs/board.md` and recording its output on the Issue.

| Skill ID | Stage | Inputs | Outputs | Done when |
|---|---|---|---|---|
| `clarification` | 1–3 | A draft file in `drafts/`, notion text from the owner, or an existing Issue in Inbox | If no Issue exists: Issue created with the notion verbatim and the type label. From a draft: the body also records the draft path, the file is removed (or moved under `docs/` when it is also a repository document) in an intake commit naming the Issue, after checking no Issue already names that path. Questions as one Issue comment, or a "no questions" note. Status → Needs your answer or Working | Every answer is recorded on the Issue in a decisions comment; no taken draft remains in `drafts/` |
| `specification` | 4 | Issue, clarification Q&A | Spec comment in plain language: goal, non-goals, acceptance criteria, test plan (human steps + expected results), testability needs. Approval request. Status → Needs your answer | Owner comments `approved` |
| `testability-preflight` | 5 | Approved spec, `docs/validator-capabilities.md` | Gap list (needed vs. already proven). Owner request for anything only the owner can give. Validator launched in preflight mode. Result comment. Capabilities registry updated | Preflight pass recorded; or `owner-test-only approved`; or a separate capability Issue created and this item waits on it |
| `planning` | 6 | Approved spec | Plan comment: approach, affected areas, sequence, risks, deferred items. Asks the owner only if a choice changes product behavior | Plan comment posted |
| `tasks` | 7 | Plan | Sub-issues (label `task`) with done-criteria, attached as GitHub sub-issues of the parent. Summary comment listing them | All sub-issues exist, none is on the board |
| `implementation` | 8 | Sub-issues, branch naming rule | Branch `feat/<issue>-<slug>` or `fix/<issue>-<slug>`; commits per sub-issue; tests. Sub-issues may use auto-close keywords | Every sub-issue is implemented and local checks pass |
| `pull-request` | 9, 11 | Branch | PR from the template: `Related to #N`, sub-issue task list, no auto-close keyword for the parent. Later: merge gate check (CI green, validator pass for the head SHA, keyword check), squash merge, branch deleted | PR merged; hands off to `present-for-owner-testing` |
| `technical-validation` | 10 | Open PR | CI status and test results summarized on the PR. `visual-validation` run if any user-visible UI change. Then the validator is launched with its three inputs and its verdict is confirmed on the Issue | CI green, results posted, validator verdict recorded for the head SHA |
| `visual-validation` | 10 | PR preview, spec | Each user-visible change checked against the spec's expected results; evidence posted on the PR | Evidence posted; discrepancies fixed or listed |
| `correction` | 13 → 7 | Owner's problem report (Needs correction) or validator fail | Round counter on the Issue. Correction sub-issues, new `fix/` branch and PR, same path. After two failed owner rounds or two consecutive validator fails: summary comment, status → Needs your answer | Correction PR merged → Ready to test, or owner decision recorded |
| `present-for-owner-testing` | 12–13 | Merged PR, spec test plan, test location | Issue comment in plain terms: where to test, steps, expected results. Status → Ready to test. On the owner's verdict: Done (Issue closed) or Needs correction (hand off to `correction`). `abandoned` → Done with label | Comment posted and status moved |

Shared reference used by all skills: `docs/board.md` (status moves, sub-issue attachment, where IDs live). It is a reference file, not a twelfth skill, so the document's "one skill per stage" holds.

## Agents

### `main` (primary, default agent)

- System prompt: owns intake to completion; follows `AGENTS.md`; loads the skill for the current stage; moves board items; never posts the approval words.
- Permissions: default tool access; `subagent` denied for all except `validator` and `explore`.

### `validator` (subagent)

- System prompt: the three inputs it expects (notion verbatim, approved spec with test plan, result URL + commit SHA); preflight mode (spec + capabilities registry only); test only through the public interface; record steps, observations, and verdict; flag drift between spec and notion.
- Verdict format on the Issue, machine-readable and human-readable:

  ```text
  <!-- validator:pass sha=<full-sha> --> or <!-- validator:fail sha=<full-sha> -->
  Validator: PASS | FAIL for <sha>
  Checked: ... Evidence: ... Reasons (on fail): ... Drift from notion: none | <description>
  ```

- Permissions (ordered, last match wins): deny `edit`, deny `question`, deny `subagent`; deny `shell` then allow `gh issue view *`, `gh issue comment *`, `gh pr view *`, `gh pr diff *`, `gh pr checks *`, `curl *`; `gh api *` asks; deny reading `*.env*`. Browser tools stay available for UI checks.
- Model: inherits the session model unless `docs/board.md` records a different choice.

### Agent Temp Workspace

- Location: `tmp/agent-shared/` (gitignored, never committed). `drafts/` is never used for agent temp files.
- Writers: `main` creates, updates, and deletes directly when obsolete. `validator` never writes there (its `edit` deny covers it); it only reads files `main` names in its three inputs.
- Lifecycle: create on need, delete directly when the Issue round ends or the file is superseded. No Issue record needed; the Issue holds only the verdict, not the scratch files.

## CI

`.github/workflows/ci.yml` runs on every pull request and on every push to `main` (so intake commits are checked too):

1. **Markdown lint** on `**/*.md` with a light rule set.
2. **Workflow files check**: every `.opencode/agents/*.md` and `.opencode/skills/*/SKILL.md` has a `description`; exactly the eleven skill IDs exist; `AGENTS.md` is present.
3. **Branch name guard**: head branch matches `^(feat|fix)/[0-9]+-[a-z0-9-]+$`.
4. **Parent auto-close guard**: fails if the PR body contains `close|closes|closed|fix|fixes|fixed|resolve|resolves|resolved` followed by `#N` where Issue N does not carry the `task` label.

The job is named `ci` and is the required status check. A CI-enforced "validator pass exists for this SHA" check is a later hardening item; in v1 the `pull-request` skill performs that check before merging.

## Rule Coverage

Where each instruction from the workflow document is enforced.

| Rule | Home |
|---|---|
| Preserve the notion; ask before changing it | `AGENTS.md`; validator drift check; `clarification` and `specification` skills quote the notion verbatim |
| Stop and ask when incomplete, contradictory, or impossible | `AGENTS.md`; `planning` and `implementation` skills end with "stop → Needs your answer" |
| Merged PR → Ready to test, never Done; no auto-close for parent | `AGENTS.md`; PR template; CI parent auto-close guard; `pull-request` skill |
| No Ready to test without CI green + validator pass for the merged commit | Branch protection (CI); `pull-request` skill merge gate reads the `validator:pass sha=` marker |
| No implementation before spec approval and preflight pass | `AGENTS.md`; `implementation` skill checks for `approved` and a preflight pass comment before starting |
| Secrets never in Issues, comments, or code | `AGENTS.md`; `.env*` read denied in `opencode.jsonc`; validator checks access without echoing |
| Validator never writes or edits code | `validator.md` permissions (`edit` deny, narrow shell allowlist) |
| Approval is an explicit owner comment, never implied | `AGENTS.md` approval-words rule; skills look for the exact comment |
| Bounded correction and validator loops | `correction` skill round counter |
| Sub-issues off the board | Decision 6 filter; `tasks` skill verifies |
| Drafts taken once, verbatim, then removed; artifacts never in `drafts/` | `clarification` skill intake steps; `drafts/README.md`; Issue body records the draft path; `AGENTS.md` pointer |
| Agent temp shared files outside `drafts/`, deleted directly when obsolete | Decision 10; `tmp/agent-shared/` gitignored via `.gitignore`; Agent Temp Workspace section; `main` owns deletion, `validator` read-only |

## Work Breakdown

Each row becomes one sub-issue. Phases run in order; rows within a phase can run in parallel unless noted.

### Phase 1: Instructions, agents, configuration

| # | Task | Done when |
|---|---|---|
| 1.1 | After the take has moved the document to `docs/workflow.md`: fix internal references, update `README.md` with a one-paragraph description and a link, and point `drafts/README.md` at the document's Drafts Folder section | Links resolve; `drafts/` holds only `README.md` and untaken drafts |
| 1.2 | Write `AGENTS.md`: the seven instruction rules verbatim from `docs/workflow.md`, the approval-words rule, the drafts intake pointer (offer untaken drafts at session start), pointers to `docs/board.md` and the skill list | Content matches the document word for word for the seven rules |
| 1.3 | Write `.opencode/opencode.jsonc` with `default_agent: main` and shared permissions (`.env*` read denied) | OpenCode starts with `main` selected in this repo |
| 1.4 | Write `.opencode/agents/main.md` | Agent appears; it can launch `validator` and `explore` only |
| 1.5 | Write `.opencode/agents/validator.md` with the permission list above | A test launch cannot edit a file or run `git push`; it can post an Issue comment |
| 1.6 | Create `docs/validator-capabilities.md` with the registry format (capability, how proven, Issue, date) and no entries | File exists |
| 1.7 | Create gitignored `tmp/agent-shared/` with a `README.md` stub (temp only, delete directly when obsolete) and add it to `.gitignore` | Temp files never appear in `drafts/` or in commits |

### Phase 2: Skills (depends on 1.2)

| # | Task | Done when |
|---|---|---|
| 2.1 | `clarification`, `specification`, `planning` | Each `SKILL.md` has description, inputs, steps, outputs, done-criteria, and the status move |
| 2.2 | `testability-preflight` including how to launch the validator in preflight mode | Same |
| 2.3 | `tasks`, `implementation`, `pull-request` including the merge gate | Same |
| 2.4 | `technical-validation`, `visual-validation` including how to launch the validator with its three inputs | Same |
| 2.5 | `correction`, `present-for-owner-testing` including the round counter and acceptance handling | Same |
| 2.6 | `docs/board.md` skeleton: commands for each status move, sub-issue attachment (`POST /repos/{owner}/{repo}/issues/{parent}/sub_issues` with the sub-issue's numeric `id`, not its number; fallback is a `- [ ] #N` task list in the parent body), placeholders for IDs | Every skill's final step points at it |

### Phase 3: GitHub files in the repo

| # | Task | Done when |
|---|---|---|
| 3.1 | Issue templates `notion.yml`, `feedback.yml`, `bug.yml` (one free-text field, label applied) | Creating an Issue from each template yields the right label and verbatim body |
| 3.2 | `PULL_REQUEST_TEMPLATE.md` | New PRs open with `Related to #`, validator pass fields, and the no-auto-close warning |
| 3.3 | `.github/workflows/ci.yml` with the four checks; job name `ci` | Green on a compliant PR; red on a bad branch name or a parent auto-close keyword |

### Phase 4: GitHub configuration (owner permissions; `gh` as `taftary`)

| # | Task | Done when |
|---|---|---|
| 4.1 | Create the six labels | `gh label list` shows exactly the set |
| 4.2 | Create Project "Astrolith", link to the repo, set Status options to the six names (GraphQL `updateProjectV2Field` with `singleSelectOptions`; it overwrites the whole option set, so send all six in one call; UI fallback) | `gh project field-list` shows the six options |
| 4.3 | Configure auto-add filter (Project UI; no API) | A new `notion`-labeled Issue appears in Inbox without manual action |
| 4.4 | Branch protection on `main`: PR required, 0 approvals, `ci` required, no force push, admin bypass left on for intake commits (Decision 9) | A PR with red `ci` cannot be merged; a PR with green `ci` can; an intake commit still lands directly |
| 4.5 | Fill `docs/board.md` with project number, field ID, option IDs | A status move command from the file works |

Phase 4 happens after Phase 3 is merged, because 4.4 would otherwise block the scaffolding PRs. Until then, scaffolding merges without protection.

### Phase 5: Dry run (depends on all above)

| # | Task | Done when |
|---|---|---|
| 5.1 | Owner writes the dry-run notion (Decision 8) as a draft file; in a session the AI takes it and runs stages 1–13 using only the skills, agents, and board | Draft gone from `drafts/`; item reaches Done; every stage left its artifact on the Issue; the validator verdict names the merged SHA |
| 5.2 | Record the first proven capability ("read a rendered file on GitHub at a commit SHA") in `docs/validator-capabilities.md` | Entry present |
| 5.3 | Fix whatever the dry run exposed; update `docs/workflow.md` first if the process itself must change | No open findings |

### Deferred (separate work items, not in this plan)

- Validator environment for the first product notion: preview deployment, test accounts, seed data, browser automation, screenshot hosting.
- Dedicated GitHub identity for the AI (bot account or GitHub App) so approval comments are verifiably the owner's.
- CI-enforced validator-pass check keyed on the `validator:pass sha=` marker.
- Event-driven trigger (run on Issue or comment events) replacing session-driven intake.

## Sequencing

```text
Phase 1 ─► Phase 2 ─┐
                    ├─► merge scaffolding ─► Phase 4 ─► Phase 5
Phase 3 ────────────┘
```

Phases 1–3 are one feature branch and one PR (`feat/<N>-workflow-scaffolding`, where `N` is the parent Issue number) because the pieces are only testable together. Phase 4 is configuration, recorded on the Issue. Phase 5 is the acceptance test of the whole plan.

## Risks

| Risk | Mitigation |
|---|---|
| Shared GitHub identity lets the AI write approval words | Approval-words rule in `AGENTS.md`; bot identity deferred item |
| Shell permission matching is best-effort text matching | Validator uses deny-by-default with a short allowlist; it has no reason to run anything else |
| Auto-add filter and Status options are partly UI-only | Owner does two one-time UI steps; `docs/board.md` records the result so it never needs repeating |
| A validator pass goes stale after a new commit | Marker carries the SHA; merge gate compares it to the PR head |
| The eleven skills drift from `docs/workflow.md` over time | Governance rule: document changes first, same PR; CI checks the skill set exists |
| Session-driven model means nothing moves while no session is open | Stated as Decision 2; event-driven trigger deferred |
| Intake commits bypass branch protection | They touch only `drafts/` and the one-time move; CI runs on push to `main`; the Issue is the record, so a bad intake commit loses nothing |
| Temp shared files deleted directly | They live only in gitignored `tmp/agent-shared/`; loss is harmless because the Issue holds the verdict and `main` recreates them on need |
| A draft is taken while the owner is still writing it | Drafts are offered, not taken silently; the owner confirms which ones at session start |

## Done Criteria for This Plan

- All deliverables exist and CI is green on `main`.
- The dry run (5.1) reached Done through every stage with no manual status moves by the owner.
- Every row of Rule Coverage points at something that exists.
- `docs/workflow.md`, `AGENTS.md`, agents, and skills do not contradict each other.

## Questions for the Owner

All answered 2026-10-01 (confirmed defaults). Decision 10 (`tmp/agent-shared/`, Option A) also confirmed.

1. Session-driven v1 (Decision 2): confirm, or ask for the event-driven trigger to be pulled into this plan.
2. The workflow document's permanent home is `docs/workflow.md` after the take (Decision 4): confirm, or name another location.
3. Dry-run notion (Decision 8): use the README notion, or name a different small notion.
4. Bot identity (Decision 3): defer as planned, or set up a bot account before the dry run.
5. Intake commits directly to `main` (Decision 9): confirm, or require a small PR per draft.
