# Requirements — issue #90 (CONFIRMED)

Derived by the specification skill from approved Spec v1
(https://github.com/taftary/astrolith/issues/90#issuecomment-5966453635,
approval: https://github.com/taftary/astrolith/issues/90#issuecomment-5966456784).

Owner approval of the spec confirms the criteria text below. Probes are the
AI's addition; the owner never sees this syntax.

## Layer 1 criteria (AC1-AC5)

- [ ] C1: Running create-branch creates and checks out the prefixed branch, and the command's own output names that same branch, for feat and fix kinds (verify: file scripts/sidebar/development.py contains --name", full)
- [ ] C2: A slug that already begins with the issue number still yields one prefixed branch, not a doubled prefix (verify: file scripts/sidebar/development.py contains a.slug.startswith)
- [ ] C3: Re-running create-branch for an existing branch reports already_present with the prefixed name, with guard and creation using the same string (verify: file scripts/sidebar/development.py contains already_present)
- [ ] C4: list shows branches linked to the issue through the supported mechanism and never silently returns an empty list when the call fails (verify: file scripts/sidebar/development.py contains --list)
- [ ] C5: No other file changes. Only scripts/sidebar/development.py is touched (verify: file scripts/sidebar/development.py contains l.split()[0])
