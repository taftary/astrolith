# Requirements (CONFIRMED) — issue #474

Derived by the specification skill from approved Spec v1
(https://github.com/taftary/astrolith/issues/474#issuecomment-6104863896,
approval: https://github.com/taftary/astrolith/issues/474#issuecomment-6104879390).
One criterion per acceptance criterion, plus one per owner-expectation sentence
from the notion, in owner words. Owner approval of the spec is confirmation of
this criteria text.

C1-C10 restate acceptance criteria 1-10 of Spec v1. C11-C12 both derive from
acceptance criterion 11 (no new gate script and unchanged gate on one side,
passing suites on the other). C13-C14 are the owner-expectation sentences of
the notion: the settings-tool wish from the Issue body and the game-guides
wish from the owner addition comment
(https://github.com/taftary/astrolith/issues/474#issuecomment-6103631944).

- [ ] C1: The specification skill has a step, run before the spec is written, that checks the item for Settings candidates and Guides candidates using the three questions above. (verify: file .claude/skills/specification/SKILL.md contains Settings and Guides candidates)
- [ ] C2: The step only looks at what the item adds or changes, and says so. It does not sweep the whole game. (verify: file .claude/skills/specification/SKILL.md contains only what this item adds or changes)
- [ ] C3: The skill describes the Settings candidate format: number (S1...), name, proposed place (Category > Sub-category), choices, default, a one-line reason, and the question Add to Settings?. (verify: file .claude/skills/specification/SKILL.md contains Add to Settings?)
- [ ] C4: The skill describes the Guides candidate format: number (G1...), proposed page, the exact text in full, and for wrong existing text both the current and the new text. (verify: file .claude/skills/specification/SKILL.md contains current text and the new text)
- [ ] C5: The skill tells the agent to take the current categories and pages from the game itself, not from memory. (verify: file .claude/skills/specification/SKILL.md contains crates/universe-render/src/panel.rs)
- [ ] C6: The skill allows a new category or page, marked (new category), and says a yes also creates it. (verify: file .claude/skills/specification/SKILL.md contains (new category))
- [ ] C7: The skill requires both sections in every spec, with None: this item changes nothing a player sees. for items without a player-facing change. (verify: file .claude/skills/specification/SKILL.md contains None: this item changes nothing a player sees.)
- [ ] C8: The skill describes the answer flow: answers first in one comment (S1 yes, G1 no), then the next spec version with yes in scope and no under non-goals, then approved. Without candidates the approval request goes out directly. (verify: file .claude/skills/specification/SKILL.md contains S1 yes, G1 no)
- [ ] C9: The skill says what happens when approved arrives with unanswered candidates: the agent asks for the answers, and the next version needs a new approved. (verify: file .claude/skills/specification/SKILL.md contains with unanswered candidates)
- [ ] C10: docs/workflow.md names the two candidates sections in the Specification artifact and the answer-then-approve order. (verify: file docs/workflow.md contains Settings candidates and Guides candidates, answered before approval)
- [ ] C11: No new gate script. The spec gate still requires the five sections including acceptance criteria. (verify: file scripts/gates/spec_gate.py contains acceptance criteria)
- [ ] C12: The existing gate fixture suites and the full workspace test suite still pass. (verify: contains test result: ok in cargo-test.log)
- [ ] C13: The owner asked for a tool that detects a tunable value and asks to add it to Settings. The skill now does that detection. (verify: file .claude/skills/specification/SKILL.md contains tunable value)
- [ ] C14: The owner asked for a guides step that proposes the full Guides text for new behaviour. The skill now does that. (verify: file .claude/skills/specification/SKILL.md contains full text)
