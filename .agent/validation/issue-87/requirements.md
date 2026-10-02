# Requirements — issue #87 (CONFIRMED)

Derived by the specification skill from approved Spec v6
(https://github.com/taftary/astrolith/issues/87#issuecomment-5961253222,
approval: https://github.com/taftary/astrolith/issues/87#issuecomment-5961269405).
Spec v1 through v5 are superseded.

Owner approval of the spec confirms the criteria text below. Process-layer
criteria use file probes (worktree reads); runtime-layer criteria use log
probes. Probes are the AI's addition; the owner never sees this syntax.

## Scope of this file, and why it holds only Layer 1

Spec v6 splits its acceptance criteria by *who can discharge them*, because the
validator must not be asked for MET n/n on criteria that cannot exist before a
merge. History, so the shape is not mistaken for arbitrary trimming:

- Spec v3 had one flat list of 11. The validator returned MET 3/11 and
  `merge_gate.py:143` refused. Four of the eight gaps were criteria asserting on
  a merge commit that does not exist while the pull request is open.
- Spec v5 layered them, but left AC7 — "the pull-request run is green at the head
  SHA" — in Layer 1. That cannot be true until a verdict exists, because the
  verdict is what unblocks the run. The validator ruled FAIL at MET 6/7 on
  exactly that criterion and correctly refused to transfer a green run from a
  prior head whose only differing input was the PR body.
- Spec v6 removes the circularity at the root: CI-greenness-at-head is assigned
  to `merge_gate.py:123`, which already performs that check at merge time. It is
  **not** in this file. It is not dropped — the gate refused this very pull
  request on that ground at head `e9c5aa1`
  (`ci latest state is 'FAILURE', not SUCCESS`), which is the evidence that the
  check lives there and works.

This file therefore holds **only Layer 1, the eight criteria the validator can
discharge on the pull request before merge**. Layer 2 (AC9-AC11, asserted on the
merge commit's `main` run) and Layer 3 (AC12-AC13, owner-only) are enumerated in
the approved spec and discharged after merge and during owner testing.

That is deliberate, not an omission. Every criterion in this file is counted by
`merge_gate.py` against the verdict's MET n/n, so a criterion the validator
structurally cannot close must not sit here — leaving it in is what made #87
unmergeable through five spec versions.

## What these probes do and do not prove

This Issue changes continuous-integration configuration and one documentation
registry, not application runtime. `validate.py` can only read worktree files and
its three runtime logs, so every probe below is a **mechanism** probe: it proves
the mechanism that makes the criterion true is present and unaltered in the file.
It does not prove the CI run's observed outcome.

The runtime half of each criterion — per-job and per-step conclusions, check-run
annotations, the workflow-file diff scope — is established by the validator
subagent from the GitHub API and recorded verbatim in its verdict comment on the
Issue, which is the record the merge gate reads alongside this report. Same
convention as issue #76. Stated plainly so no reader mistakes a file-string match
for a CI observation.

No criterion asserts on CI step-log text. Step-log reachability is
credential-dependent — HTTP 200 with admin rights, 403 without — and the standing
owner decision of 2026-10-02 is not to rely on it, so no criterion here depends on
it either way.

C7 and C8 are registry corrections to `docs/validator-capabilities.md`, which rides
in this diff because that path is outside the Intake path guard's allowed tuple
and therefore cannot reach `main` by direct push. Both were flagged by the
validator as wrong at this head; the underlying facts were verified
independently before being recorded.

## Layer 1 criteria (AC1-AC8)

- [ ] C1: The `ci` job's checkout brings enough history that the guard can resolve a parent commit, and `ci-windows`'s checkout is unchanged (verify: file .github/workflows/ci.yml contains fetch-depth: 0)
- [ ] C2: A self-test step exists in the `ci` job, is not gated by any `if:`, and on the pull-request run its conclusion is `success` and not `skipped` (verify: file .github/workflows/ci.yml contains - name: Intake path guard self-test)
- [ ] C3: The self-test's body asserts all three of: the checkout resolves a parent commit; a push touching only `drafts/`, `docs/workflow.md` or `.agent/` passes; a push touching any other path is rejected. Each assertion exits non-zero on failure (verify: file .github/workflows/ci.yml contains SELFTEST-OK)
- [ ] C4: The guard step's body in `.github/workflows/ci.yml` is byte-identical to its state at the fix base — same merge skip, same `allowed` tuple, same `sys.exit(1)` on the bad-list path (verify: file .github/workflows/ci.yml contains allowed = ("drafts/", "docs/workflow.md", ".agent/"))
- [ ] C5: The self-test takes its `allowed` tuple by parsing it out of the guard's own body rather than hard-coding a copy, so narrowing or widening the guard is followed instead of silently passed over (verify: file .github/workflows/ci.yml contains tuple(re.findall()
- [ ] C6: The diff of `.github/workflows/ci.yml` between fix base and head is a pure addition with zero deletions, touching only the `ci` job's checkout and the added self-test step. No `crates/` file, no `Cargo.toml` and no lockfile changes. Probed here on the pre-existing step's condition surviving; the zero-deletion scope is established by the validator from the compare API (verify: file .github/workflows/ci.yml contains if: github.event_name == 'push')
- [ ] C7: `docs/validator-capabilities.md` records step-log reachability as credential-dependent — reachable with admin rights, 403 without — and no longer claims flat 403 as a universal limit (verify: file docs/validator-capabilities.md contains credential limit, not a universal one)
- [ ] C8: `docs/validator-capabilities.md` states the `--verify` determinism position without contradicting `validate.py`: raw byte-identity disputed and unproven, normalized repeat identity asserted by the script as a hard error, with the masking of PIDs and timings named (verify: file docs/validator-capabilities.md contains stricter than the evidence warrants)
