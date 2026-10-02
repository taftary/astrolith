# muse-review-lifecycle

Authored by the agent at the owner's request, 2026-10-02. This is an independent review of the workflow lifecycle with primary focus on the lifecycle state machine, agent permissions, and workflow documentation: `docs/workflow.md`, `docs/board.md`, `AGENTS.md`, `.opencode/agents/main.md`, `.opencode/agents/validator.md`, the 12 skills under `.opencode/skills/`, `docs/plans/workflow.md`, `docs/validator-capabilities.md`, and `.agent/project-config.json`. Taking this draft follows the normal intake rule: the Issue body holds this file verbatim plus this path, then the file is removed.

Scope: the lifecycle machinery only (workflow stage-to-skill mapping, agent contracts and permissions, board transitions, acceptance vocabulary, and doc consistency). The universe product itself is untouched by this review.

## What I read

- Core governance: `docs/workflow.md` (source of truth), `docs/board.md`, `AGENTS.md`, `drafts/README.md`.
- Agent definitions: `.opencode/agents/main.md`, `.opencode/agents/validator.md`, `.opencode/opencode.jsonc`.
- All 12 workflow skills: `clarification`, `specification`, `testability-preflight`, `planning`, `tasks`, `implementation`, `pull-request`, `technical-validation`, `visual-validation`, `correction`, `present-for-owner-testing`, `wait-checks`.
- Supporting artifacts and scripts: `docs/plans/workflow.md`, `docs/validator-capabilities.md`, `.agent/project-config.json`, `.github/PULL_REQUEST_TEMPLATE.md`, `.github/workflows/ci.yml`, `scripts/sidebar/*.py`, `scripts/validation/validate.py`.

## Verdict in one line

The lifecycle architecture is thoughtfully designed, but the operational link between workflow stages and skills is undocumented, five skills cite an obsolete plan rather than live governance, the validator agent's permissions directly conflict with its stated duties, and board status transitions contain unhandled contradictions.

## Findings, ranked

1. **The 13-stage sequence lacks an explicit mapping to the 12 skills.**
   `docs/workflow.md:90-104` defines 13 stages. However, `docs/workflow.md:226` and `AGENTS.md:26` list only 11 skills, entirely omitting `wait-checks`. In reality, `clarification` spans stages 1-3, `specification` handles stage 4, `testability-preflight` handles stage 5, `planning` handles stage 6, `tasks` handles stage 7, `implementation` handles stage 8, `pull-request` spans stages 9 and 11, `technical-validation` and `visual-validation` share stage 10, `present-for-owner-testing` covers stages 12 and 13, `correction` re-enters at stage 7, and `wait-checks` acts as a utility sub-skill with no designated stage. This relationship is defined nowhere in the documentation.

2. **Five skills point to `docs/plans/workflow.md` as an active reference.**
   `clarification/SKILL.md:11`, `implementation/SKILL.md:8`, `technical-validation/SKILL.md:8`, `pull-request/SKILL.md:8`, and `correction/SKILL.md:8` cite the Skills table in `docs/plans/workflow.md`. That document was the historical plan for Issue #2 and still references obsolete status names (`Inbox`, `Working`, `Ready to test`) and states there are only 11 skills. The 2026-10-01 rename to `Todo`, `In progress`, and `In review` (`docs/board.md:9`, `.agent/project-config.json:33`) is not reflected there, creating cognitive drift.

3. **Validator duties contradict agent permissions and tool grants.**
   In `.opencode/agents/validator.md`:
   - Line 5-7 denies `edit: *`, but line 73 orders the validator to write evidence under `.agent/validation/issue-<n>/<timestamp>/` and update `.agent/validation/known-misses.md`.
   - Line 60-61 sets `gh api *` to `effect: ask`. Because the validator subagent runs headless with `question: *` denied (lines 8-10), any API call blocks or prompts interactively, contradicting line 67 which mandates reading GitHub files at a commit SHA via the API.
   - Lines 69-71 mandate scanning "browser console, container logs" for errors, but the allowlist (`:17-58`) grants no browser tool, driver, or container access.

4. **Status move rules contain direct contradictions across files.**
   - `docs/board.md:38`, `docs/workflow.md:203`, and `.agent/project-config.json:24` mandate that any validator FAIL routes to `In progress`. In contrast, `correction/SKILL.md:22` states that a validator FAIL during a correction round keeps the item in `Needs correction`.
   - `docs/board.md:3` requires that every skill end with a board move. Yet `wait-checks` performs no move, and `visual-validation/SKILL.md:20` says "No status change" while line 30 says status is "handled per docs/board.md".
   - `docs/board.md:42` defines a transition `Work paused or abandoned -> Back to Todo`, whereas `docs/workflow.md:172,193` mandates that abandoned work moves to `Done` with reason `abandoned`, and no `Todo` return path exists in `docs/workflow.md`.

5. **Loop bounds are inconsistent between governance and skills.**
   `docs/workflow.md:174` and `correction/SKILL.md:18` specify a hard stop after two consecutive validator fails on an item. However, `technical-validation/SKILL.md:30` specifies "Maximum 3 validator loops" before stopping.

6. **Post-merge validation has no executing owner or skill.**
   `docs/board.md:41` and `.agent/project-config.json:27` specify `mergedPostMergePass -> In review`. `docs/workflow.md:144,204` states that after merge, the merged result with post-merge validation awaits owner testing. However, neither `pull-request` nor `present-for-owner-testing` runs `validate.py` on the merged commit on `main`, leaving post-merge validation unperformed in practice.

7. **Prose rules forbid hardcoding statuses while all skills use hardcoded literals.**
   `docs/board.md:7` strictly commands: "Never hardcode Status names or IDs in skills or scripts; read them from that file [.agent/project-config.json]." Yet every single skill specifies literal CLI strings such as `--status "In progress"` or `--status "Needs your answer"`, and `project.py` does not provide an option to set status by lifecycle key.

8. **Missing definition and enforcement for owner acceptance vocabulary.**
   Approval words (`approved`, `owner-test-only approved`, `abandoned`) are explicitly documented in `AGENTS.md` and denied to the AI. However, the owner's acceptance word for marking an item `Done` is neither defined as an exact token in `AGENTS.md` nor in `.agent/project-config.json` (Issue #63 recorded `accept`), and no agent rule explicitly forbids the AI from emitting the acceptance token.

9. **The validator launch contract is unstructured.**
   `docs/workflow.md:245` defines the three mandatory inputs for the validator gate (notion verbatim, approved spec with test plan, PR preview URL + commit SHA), and two inputs for preflight (`spec` + `docs/validator-capabilities.md`). However, no automated validation checks the brief, so malformed briefs result in failed runs or confusion rather than a clean, immediate `BLOCKED` verdict.

10. **The capabilities registry is narrative prose rather than a formal verifiable record.**
    In `docs/validator-capabilities.md`, past preflight capabilities are logged as descriptive sentences in a markdown table. Without a structured format linking each capability to a concrete command or verification script, the preflight step cannot programmatically verify what is already proven.

## Requirements

### A. Stage-to-Skill Mapping and Documentation Consistency

- **M1** Update `docs/workflow.md` to include a complete, explicit table mapping all 13 workflow stages to their corresponding skills across all 12 skills:
  - Stages 1-3: `clarification`
  - Stage 4: `specification`
  - Stage 5: `testability-preflight`
  - Stage 6: `planning`
  - Stage 7: `tasks`
  - Stage 8: `implementation`
  - Stage 9: `pull-request` (opening)
  - Stage 10: `technical-validation` and `visual-validation`
  - Stage 11: `pull-request` (merge gate & merge)
  - Stage 12: `present-for-owner-testing`
  - Stage 13: `present-for-owner-testing` (verdict) / `correction` (re-entry to stage 7)
  - Mark `wait-checks` as a utility sub-skill used within `pull-request`.
- **M2** Mirror the full list of 12 skills and stage mappings in `AGENTS.md`, and update CI in `.github/workflows/ci.yml` to verify that every skill directory is documented in `docs/workflow.md` and `AGENTS.md`.
- **M3** Mark `docs/plans/workflow.md` with a prominent historical archive header stating it reflects the initial setup of Issue #2. Repoint `clarification`, `implementation`, `technical-validation`, `pull-request`, and `correction` away from `docs/plans/workflow.md` to `docs/workflow.md` and `docs/board.md`.

### B. Validator Permissions and Contract Alignment

- **M4** Update `.opencode/agents/validator.md` permissions to match operational requirements:
  - Allow read-only `gh api *` commands so the validator can inspect repository contents and commits without interactive prompts.
  - Clarify file write permissions for writing evidence under `.agent/validation/issue-<n>/<timestamp>/` and `.agent/validation/known-misses.md` while strictly maintaining the denial of code modifications in repository source paths.
  - Align log inspection requirements with available tools, removing browser console scanning requirements unless a headless browser or capture tool is explicitly provided.
- **M5** Formalize the briefing contract in `technical-validation` and `testability-preflight`: if any mandatory input is missing or empty, the validator immediately outputs a `BLOCKED` report specifying the exact missing input without attempting execution.

### C. Reconciled Status Transitions and Board Lifecycle

- **M6** Reconcile the validator FAIL transition: update `correction/SKILL.md` to align with `docs/board.md` and `docs/workflow.md`, ensuring that a validator FAIL consistently routes the parent Issue to `In progress`.
- **M7** Standardize the validator retry bound to exactly 2 consecutive fails across all skills (`technical-validation/SKILL.md` updated from 3 to 2), matching `docs/workflow.md:174`.
- **M8** Document the explicit exceptions to the "every skill moves status" rule in `docs/board.md`: `wait-checks` (moves no status; caller manages status) and `visual-validation` (evidence posted; status transition handled by `technical-validation`).
- **M9** Resolve the `Back to Todo` transition in `docs/board.md:42`: align with `docs/workflow.md` so that abandoned items move to `Done` (reason `abandoned`), and remove or document any valid transition to `Todo`.
- **M10** Establish a formal post-merge validation procedure: update `pull-request` and `present-for-owner-testing` to run `python scripts/validation/validate.py --issue N --sha <merge-commit-sha>` on `main` following merge, post the report, and transition the item to `In review`.

### D. Machine-Readable Configuration and Vocabulary

- **M11** Add the official owner acceptance word (e.g., `accepted` or `accept`) to `.agent/project-config.json` alongside `approvalWords`. Add this acceptance token to `AGENTS.md` and `.opencode/agents/main.md` as forbidden for the AI to emit.
- **M12** Enhance `scripts/sidebar/project.py set-status` to support `--status-key <lifecycleKey>` referencing `.agent/project-config.json.lifecycle`, and update skills to use lifecycle keys rather than hardcoded display strings.

## Decisions needed from the owner

1. **Acceptance keyword**: Confirm whether the official owner acceptance token for completing an item to `Done` is `accepted` or `accept` (M11).
2. **Validator retry limit**: Confirm that 2 consecutive validator fails is the definitive threshold to pause and request owner intervention (M7).
3. **Paused work state**: Clarify whether pausing active work is permitted to return an Issue to `Todo`, or if all paused work remains in `Needs your answer` / `In progress` (M9).
4. **Post-merge validation enforcement**: Confirm whether post-merge validation on `main` is required for every parent Issue before owner presentation (M10).

## Non-goals

- Altering the core universe product or crates under `crates/`.
- Changing the six fundamental board statuses (`Todo`, `Needs your answer`, `In progress`, `In review`, `Needs correction`, `Done`).
- Introducing automated event-driven daemon webhooks outside interactive sessions.

## Done when

- `docs/workflow.md` and `AGENTS.md` feature the complete 13-stage to 12-skill mapping.
- Five skills have their references updated from `docs/plans/workflow.md` to live documents.
- Validator permissions in `.opencode/agents/validator.md` permit read-only API access and match documented duties.
- Contradictions in loop bounds, FAIL transitions, and board moves are reconciled across `docs/board.md`, skills, and `.agent/project-config.json`.
- Post-merge validation is integrated into the post-merge procedure.
