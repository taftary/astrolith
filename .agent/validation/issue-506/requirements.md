# Requirements (CONFIRMED) — issue #506

Derived by the main agent from approved Spec v2
(https://github.com/taftary/astrolith/issues/506#issuecomment-6119992446,
owner approval https://github.com/taftary/astrolith/issues/506#issuecomment-6120036408).
One criterion per acceptance criterion, plus one per owner-expectation sentence
from the notion, in owner words. Owner approval of the spec is confirmation of
this criteria text.

- [ ] C1: When several validator verdict comments exist for the same commit, both merge checks use the newest one and ignore any comment marked validator:superseded (verify: file scripts/gates/merge_gate.py contains select_verdict)
- [ ] C2: A verdict block contains no Markdown heading line, and a verdict containing one fails instead of merging (verify: file scripts/gates/compare_verdict.py contains must not contain headings)
- [ ] C3: The verdict layout is written down in the validator instructions: marker line, Validator line, Checked and Evidence lines, Runtime line, Requirements MET line with one line per criterion, then Scope and Note lines, then the closing Drift line (verify: file .opencode/agents/validator.md contains Verdict layout)
- [ ] C4: When the spec itself records an owner-approved change from the original idea, the verdict still says Drift: none and names the approval in a Drift from notion line, and only unapproved drift blocks the merge (verify: file scripts/validation/post_verdict.py contains Drift from notion)
- [ ] C5: Every case is proven by unit tests with saved example text and no network, running in CI with the other script tests (verify: file .github/workflows/ci.yml contains test_compare_verdict_fixtures)
- [ ] C6: Both merge checks agree on the same newest verdict for saved comment text, with the PR block byte-identical to the Issue block (verify: file scripts/gates/test_compare_verdict_fixtures.py contains pr-clean-matches-issue)
