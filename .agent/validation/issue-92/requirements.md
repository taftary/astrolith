# Requirements — issue #92 (CONFIRMED)

Derived by the specification skill from approved Spec v1
(https://github.com/taftary/astrolith/issues/92#issuecomment-5966523183,
approval: https://github.com/taftary/astrolith/issues/92#issuecomment-5966527903).

Owner approval of the spec confirms the criteria text below. Probes are the
AI's addition; the owner never sees this syntax.

## Layer 1 criteria (AC1-AC5)

- [ ] C1: The specification skill carries the rule where criteria are authored: Layer 1 criteria must be dischargeable before any verdict exists, and run observations are gate-owned and out of the count (verify: file .opencode/skills/specification/SKILL.md contains gate-owned)
- [ ] C2: An automated check scans a criteria file and refuses the forbidden shape, naming the offending criterion number and the matched phrase (verify: file scripts/validation/validate.py contains unsatisfiable)
- [ ] C3: The check runs inside validate.py before grading so a violation fails the run with the reason printed, and it is also directly runnable on any file (verify: file scripts/validation/validate.py contains check_criteria_shape)
- [ ] C4: Issue #87's Spec v7 criteria file passes the check clean while a fixture file with a run-conclusion criterion is refused by name (verify: file .agent/validation/issue-87/requirements.md contains not gated by any)
- [ ] C5: Only the specification skill, validate.py, and one new fixture file change (verify: file .agent/validation/issue-92/fixture-run-conclusion.md contains run-conclusion criterion)
