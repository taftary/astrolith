---
name: specification
description: Use when writing the owner-facing specification and test plan for a parent Issue at workflow stage 4, before any testability preflight or implementation.
---

# Specification

## Inputs

- Parent Issue, including the notion verbatim and the clarification Q&A.
- `docs/workflow.md` (Specification artifact, Testability Check inputs).

## Steps

1. Read the Issue notion verbatim and all clarification answers. Stop and ask the owner if the notion is incomplete, contradictory, or technically impossible. Never change the notion without asking.
2. Check Settings and Guides candidates before writing the spec. This Settings and Guides candidates step runs before the spec is written. Ask three questions about only what this item adds or changes (never a whole-game sweep):
   - Does this add a tunable value a player could reasonably want to change (a speed, a brightness, an on/off, a limit)? -> Settings candidate.
   - Does this add a key, a control or a behaviour a player needs explained? -> Guides candidate.
   - Would this change make any existing Guides text wrong (for example a key moves)? -> Guides candidate showing the current text and the new text.
   Read the current categories and pages from the game itself, not from memory: `crates/universe-render/src/panel.rs` (`SETTINGS_TREE`, `GUIDES_TREE`), `crates/universe-core/src/settings.rs` (`Category`), `crates/universe-core/src/guides.rs` (`GuidePage`).
   Settings candidates are numbered S1, S2, ... Each names the setting, the proposed place as Category > Sub-category, the choices, the default, a one-line reason, and ends with the question Add to Settings? Example:
   > **S1. Free-flight acceleration**: under Controls > Flight, slider 0.5 to 2.0, default 1.0. Why: this item adds acceleration to free flight. Add to Settings?
   Guides candidates are numbered G1, G2, ... Each names the proposed page, gives the exact words in full text, and for wrong existing text shows both the current text and the new text, ending with Add to Guides? Examples:
   > **G1. Free-flight acceleration**: add to Help > Free flight: "Your speed builds up gently when you start moving." Add to Guides?
   > **G2. Navigation text is now wrong**: Help > Navigation says "E / Shift+E: brighter / darker"; this item moves exposure to B. New text: "B / Shift+B: brighter / darker". Change it?
   A candidate may propose a category or page that does not exist yet, marked (new category). A yes also creates it.
   Every spec carries both Settings candidates and Guides candidates sections. When the item changes nothing a player sees, both sections say: None: this item changes nothing a player sees. When the item touches the game but nothing qualifies, the section says None with one line saying why.
3. Write the spec in plain non-technical language as one Issue comment (or a linked `spec.md` under `docs/` only when a comment would be unwieldy) with these sections: goal, non-goals, Settings candidates, Guides candidates, acceptance criteria, test plan (human steps plus expected results for each step), testability needs (environment, accounts, seed data, third-party services). Number it `Spec v<k>` (first is v1); a new version voids the previous approval and needs a new `approved`.
4. Post an approval request on the Issue in plain language. If the spec has candidates, ask for answers first in one comment, for example `S1 yes, G1 no`. Then post Spec v(k+1) with each yes written into the goal, acceptance criteria and test plan, and each no listed under non-goals, showing the answers in the candidates sections. Then ask for `approved`. If both sections say None, ask for `approved` at once, as before. If `approved` arrives with unanswered candidates, ask for the answers; the next version needs a new `approved`, as any new version does. Approval is an explicit owner comment whose trimmed body is exactly `approved` (record the comment URL); never implied. The AI never writes the approval or acceptance words itself.
5. After approval, write `.agent/validation/issue-<n>/requirements.md` from the approved spec: one criterion per acceptance criterion plus one per owner-expectation sentence, in the owner's words, with the AI adding the `(verify: ...)` probe. Owner approval of the spec counts as confirmation of the criteria text; the owner never sees probe syntax. Commit it with the feature PR (it is part of the specification).
   - Layer-1 criteria must be dischargeable before any verdict exists. Never write a criterion that asserts a pull-request run observation (a step/check conclusion, an annotation, CI greenness at a SHA): such a criterion is unsatisfiable by construction, because the run cannot go green until the verdict the count is built from exists. Run observations are gate-owned (checked by `merge_gate.py` and recorded in the validator's verdict comment) and stay out of the `MET n/n` count. `validate.py` refuses this shape; see `check_criteria_shape`.
6. Final step: record the output on the Issue and set Status via `python scripts/sidebar/project.py set-status --issue N --status-key blocked` (waiting for approval; verified by re-read). End with the one-glance summary per `docs/board.md`.

## Outputs

- Spec comment in plain language: goal, non-goals, Settings candidates, Guides candidates, acceptance criteria, test plan (human steps + expected results), testability needs.
- Approval request on the Issue, or candidates answers request first when candidates exist.

## Done criteria

- Done only when the owner answers any candidates and then comments `approved` on the Issue. Without candidates, done when the owner comments `approved`. The spec is then frozen before implementation.
