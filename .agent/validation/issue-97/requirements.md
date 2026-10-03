# Requirements — issue #97 (CONFIRMED)

Derived by the specification skill from approved Spec v1
(https://github.com/taftary/astrolith/issues/97#issuecomment-5966693858,
approval: https://github.com/taftary/astrolith/issues/97#issuecomment-5966699800).

Owner approval of the spec confirms the criteria text below. Probes are the
AI's addition; the owner never sees this syntax.

## Layer 1 criteria (AC1-AC5)

- [ ] C1: validator.md gains explicit allows for git show, git diff, git cat-file and gh issue develop, each in the file's existing one-entry-per-line form (verify: file .opencode/agents/validator.md contains git show *)
- [ ] C2: Every new entry is exercised by a probe run after the change with each result recorded on this Issue (verify: file .opencode/agents/validator.md contains gh issue develop *)
- [ ] C3: The pre-change baseline denials for those same four commands are linked so before and after can be compared, and the default-deny posture is unchanged (verify: file .opencode/agents/validator.md contains effect: deny)
- [ ] C4: docs/validator-capabilities.md gains one row per command proven allowed by a probe, and no row for denied ones (verify: file docs/validator-capabilities.md contains #97)
- [ ] C5: Only .opencode/agents/validator.md and docs/validator-capabilities.md change, verified by diff (verify: file .opencode/agents/validator.md contains git cat-file *)
