# Requirements — issue #660 (CONFIRMED)

Derived by the specification skill from approved Spec v2
(https://github.com/taftary/astrolith/issues/660#issuecomment-6164678191,
approval: https://github.com/taftary/astrolith/issues/660#issuecomment-6164696152),
which replaced Spec v1 after the owner's review answers R1-R5.

Owner approval of the spec confirms the criteria text below. Probes are the
AI's addition; the owner never sees this syntax.

AC5 from the spec (automated checks pass, CI green) is gate-owned: it is
discharged by the CI run plus `merge_gate.py`, not by the MET count below
(run observations stay out of Layer 1 by the specification skill rule).

## Layer 1 criteria (AC1-AC4)

- [ ] C1: `docs/README.md` names every file and folder directly under `docs/` with its audience (verify: file docs/README.md contains Audience)
- [ ] C2: the new docs-vs-guides page answers without asking that a new planet fact goes to `guides/` and a new rule or decision goes to `docs/` (verify: file docs/docs-vs-guides.md contains a new planet fact goes to guides)
- [ ] C3: `docs/templates/` holds an ADR template matching the existing decision files plus a when-to-use note (verify: file docs/templates/adr.md contains ## Context)
- [ ] C4: the templates note says when to use each template (verify: file docs/templates/README.md contains when to use)
- [ ] C5: the lab doc is left to #654: it is listed in the index under its new name `docs/engine-lab.md` (verify: file docs/README.md contains engine-lab.md)
- [ ] C6: the architecture page links once to the guides universe index instead of repeating the level links (verify: file docs/ARCHITECTURE.md contains guides/universe/documents/README.md)
- [ ] C7: `docs/plans/` notes its content is historical and read-only with live plans on their Issue (verify: file docs/plans/README.md contains read-only)
- [ ] C8: the docs index links the guides index again (R2) (verify: file docs/README.md contains ../guides/README.md)
- [ ] C9: the index names the ADRs from 0001 onward, with no range to go stale (R3) (verify: file docs/README.md contains 0001 onward)
- [ ] C10: the ADR template says which parts are optional and when to use them (R4) (verify: file docs/templates/adr.md contains Optional parts)
