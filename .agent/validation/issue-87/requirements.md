# Requirements — issue #87 (CONFIRMED)

Derived by the specification skill from approved Spec v3
(https://github.com/taftary/astrolith/issues/87#issuecomment-5958073334,
approval: https://github.com/taftary/astrolith/issues/87#issuecomment-5958095973).
Spec v1 and v2 are superseded; v3's criteria are the confirmed text.

Owner approval of the spec confirms the criteria text below.
Process-layer criteria use file probes (worktree reads); runtime-layer
criteria use log probes. Probes are the AI's addition; the owner never
sees this syntax.

## What these probes do and do not prove

This Issue changes continuous-integration configuration, not application
runtime. `validate.py` can only read worktree files and its three runtime
logs, so every probe below is a **mechanism** probe: it proves the mechanism
that makes the criterion true is present and unaltered in the file. It does
not prove the CI run's observed outcome.

The runtime half of each criterion — the per-job and per-step conclusions,
the check-run annotations, the one-file diff — is established by the
validator subagent from the GitHub API and recorded verbatim in its verdict
comment on the Issue, which is the record the merge gate reads alongside
this report. That split follows the same convention as issue #76, whose
criteria are likewise all mechanism probes. It is stated here plainly rather
than left for a reader to discover.

No criterion asserts on CI step-log text. The validator cannot read step logs
(403 admin, 404 sign-in), so it uses per-job and per-step conclusions and
check-run annotations, all proven in docs/validator-capabilities.md. Note
GitHub synthesises annotation messages, so no captured output appears in one.

C5's two negative clauses are additionally exercised directly, and both
correctly fail: neutering the guard body, and a real `--depth 1` clone. That
second one matters because this working clone is full-depth, so a
parent-resolves assertion passes here vacuously even with the defect present.
No local run may be offered as evidence for C5 on its own.

## Acceptance-criterion criteria

- [ ] C1: main is green — the main run for the merge commit shows both ci and ci-windows successful (verify: file .github/workflows/ci.yml contains fetch-depth: 0)
- [ ] C2: The guard is proven to work, on the pull request itself — the guard self-test step appears on the pull-request run with a conclusion that is success and not skipped. Its body asserts all three of: the checkout can resolve the parent commit; a push touching only drafts/, docs/workflow.md or .agent/ passes the guard's check; a push touching any other path fails it (verify: file .github/workflows/ci.yml contains SELFTEST-OK)
- [ ] C3: The old failure is gone — the ci check-run for the merge commit carries no failure-level annotation, and the guard step's own conclusion is not failure. At the broken commit that check-run carried `Process completed with exit code 128` and the step read failure; both must be absent (verify: file .github/workflows/ci.yml contains fetch-depth: 0)
- [ ] C4: The guard still protects — the guard step's body in the workflow file at the merge commit still contains the allowed-path tuple ("drafts/", "docs/workflow.md", ".agent/") and still exits non-zero on a bad list, and the self-test's negative branch is green (verify: file .github/workflows/ci.yml contains allowed = ("drafts/", "docs/workflow.md", ".agent/"))
- [ ] C5: The self-test cannot pass vacuously — it fails if the checkout depth is insufficient, and it fails if the guard step's body in .github/workflows/ci.yml is removed or changed in substance (verify: file .github/workflows/ci.yml contains cannot resolve HEAD~1)
- [ ] C6: Nothing else in the workflow file changes meaning — the diff of .github/workflows/ci.yml between fix base and head touches only the checkout step of job ci and the addition of the self-test step; every pre-existing step keeps its name, condition and body. Probed here on the pre-existing steps' conditions surviving; the diff scope itself is established by the validator from the compare API (verify: file .github/workflows/ci.yml contains if: github.event_name == 'push')

## Owner-expectation criteria

The four manual checks in Spec v3's test plan, in the owner's words. C10 and
C11 are owner-side by nature and are listed so the count is honest; their
mechanism probes below only show the mechanism is in place.

- [ ] C7: Open the Actions page for the main branch head. Expected: green ci and green ci-windows (verify: file .github/workflows/ci.yml contains runs-on: ubuntu-latest)
- [ ] C8: Open the ci job of the pull request and find the guard self-test step. Expected: it is not marked as skipped, and it is green (verify: file .github/workflows/ci.yml contains - name: Intake path guard self-test)
- [ ] C9: It prints three short results: parent resolved, allowed path passed, forbidden path rejected (OWNER-ONLY: step-log text, not reachable by the validator, so this can only ever be an owner check) (verify: file .github/workflows/ci.yml contains forbidden path rejected)
- [ ] C10: Open .github/workflows/ci.yml at the change. Expected: the checkout brings enough history for the guard, and one new self-test step exists. Nothing else changed (verify: file .github/workflows/ci.yml contains fetch-depth: 0)
- [ ] C11: Nothing about the game looks different, because nothing about the game changed (verify: file crates/universe-app/src/main.rs contains --verify)
