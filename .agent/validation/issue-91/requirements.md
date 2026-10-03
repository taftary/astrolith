# Requirements — issue #91 (CONFIRMED)

Derived by the specification skill from approved Spec v1
(https://github.com/taftary/astrolith/issues/91#issuecomment-5966487145,
approval: https://github.com/taftary/astrolith/issues/91#issuecomment-5966497003).

Owner approval of the spec confirms the criteria text below. Probes are the
AI's addition; the owner never sees this syntax.

## Layer 1 criteria (AC1-AC5)

- [ ] C1: The gate finds the newest spec version, highest Spec v-k with ties to latest, and reports BLOCKED for a missing spec when no comment carries one (verify: file scripts/gates/spec_gate.py contains find_newest_spec)
- [ ] C2: An approval counts only after the newest spec comment, and the BLOCKED output names the newest unapproved version (verify: file scripts/gates/spec_gate.py contains after newest spec)
- [ ] C3: The section check runs against the newest spec comment itself, not the issue body or any older comment (verify: file scripts/gates/spec_gate.py contains spec_has_sections)
- [ ] C4: Behavior otherwise unchanged with newest spec approved plus preflight pass present reporting OPEN, and issue 87 as it stands still reporting OPEN (verify: file scripts/gates/test_spec_gate_fixtures.py contains fresh-approval)
- [ ] C5: Only scripts/gates/spec_gate.py changes, verified with a local fixture harness covering the three decisive cases (verify: file scripts/gates/test_spec_gate_fixtures.py contains superseded)
