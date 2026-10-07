# Requirements (CONFIRMED) — issue #246

Derived by the main agent from approved Spec v2
(https://github.com/taftary/astrolith/issues/246#issuecomment-6033184831,
owner approval https://github.com/taftary/astrolith/issues/246#issuecomment-6033390713).
This version voids the Spec v1 derivation: content identical, header format fixed.
One criterion per acceptance criterion, plus one per owner-expectation sentence
from the notion, in owner words. Owner approval of the spec is confirmation of
this criteria text.

- [ ] C1: The workflow file has no Windows job anymore (verify: file .github/workflows/ci.yml contains windows-removed-246)
- [ ] C2: The full test suite still runs once for pull requests and for pushes to main (verify: file .github/workflows/ci.yml contains cargo test --locked --workspace)
- [ ] C3: No separate repeat of golden tests or doc tests when the full suite already covers them (verify: file .github/workflows/ci.yml contains no-duplicate-tests-246)
- [ ] C4: Helper tools (spelling checker, dependency checker) use a cached install instead of reinstalling every run (verify: file .github/workflows/ci.yml contains cached-tools-246)
- [ ] C5: The workflow still starts on pull requests and on pushes to main (verify: file .github/workflows/ci.yml contains pull_request)
- [ ] C6: The Windows CI is gone for the moment so the PR checks stop waiting on it (verify: file .github/workflows/ci.yml contains windows-removed-246)
- [ ] C7: The test step takes less time because the duplicate runs and reinstalls are gone (verify: file .github/workflows/ci.yml contains cached-tools-246)
