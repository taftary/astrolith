# Requirements — issue #87 (CONFIRMED)

Derived by the specification skill from approved Spec v5
(https://github.com/taftary/astrolith/issues/87#issuecomment-5960912274,
approval: https://github.com/taftary/astrolith/issues/87#issuecomment-5960973754).
Spec v1 through v4 are superseded.

Owner approval of the spec confirms the criteria text below. Process-layer
criteria use file probes (worktree reads); runtime-layer criteria use log
probes. Probes are the AI's addition; the owner never sees this syntax.

## Scope of this file, and why it holds only Layer 1

Spec v5 splits its acceptance criteria into three layers by *who can discharge
them*, because Spec v3's flat list asked the validator for MET n/n on criteria
that cannot exist before a merge. It returned MET 3/11 and `merge_gate.py:143`
refused, correctly.

This file therefore contains **only Layer 1, the seven criteria the validator
can discharge on the pull request before merge**. Layer 2 (AC8-AC10, asserted on
the merge commit's `main` run) and Layer 3 (AC11-AC12, owner-only: a deliberately
shallow checkout, and step-log text) are **not** in this file.

That is deliberate, not an omission. Every criterion in this file is counted by
`merge_gate.py` against the verdict's MET n/n, so a criterion the validator
structurally cannot close must not sit here — leaving it in is precisely what
made #87 unmergeable through three spec versions. Layer 2 and Layer 3 are
enumerated in the approved spec and are discharged after merge and during owner
testing respectively. Nothing is dropped.

## What these probes do and do not prove

This Issue changes continuous-integration configuration, not application
runtime. `validate.py` can only read worktree files and its three runtime logs,
so every probe below is a **mechanism** probe: it proves the mechanism that
makes the criterion true is present and unaltered in the file. It does not prove
the CI run's observed outcome.

The runtime half of each criterion — per-job and per-step conclusions,
check-run annotations, the one-file diff scope — is established by the validator
subagent from the GitHub API and recorded verbatim in its verdict comment on the
Issue, which is the record the merge gate reads alongside this report. Same
convention as issue #76. Stated plainly so no reader mistakes a file-string
match for a CI observation.

No criterion asserts on CI step-log text. The validator cannot read step logs
(403 admin, 404 sign-in), so it uses per-job and per-step conclusions and
check-run annotations. GitHub synthesises annotation messages, so no captured
output appears in one.

## Layer 1 criteria (AC1-AC7)

- [ ] C1: The `ci` job's checkout brings enough history that the guard can resolve a parent commit, and `ci-windows`'s checkout is unchanged (verify: file .github/workflows/ci.yml contains fetch-depth: 0)
- [ ] C2: A self-test step exists in the `ci` job, is not gated by any `if:`, and on the pull-request run its conclusion is `success` and not `skipped` (verify: file .github/workflows/ci.yml contains - name: Intake path guard self-test)
- [ ] C3: The self-test's body asserts all three of: the checkout resolves a parent commit; a push touching only `drafts/`, `docs/workflow.md` or `.agent/` passes; a push touching any other path is rejected. Each assertion exits non-zero on failure (verify: file .github/workflows/ci.yml contains SELFTEST-OK)
- [ ] C4: The guard step's body in `.github/workflows/ci.yml` is byte-identical to its state at the fix base — same merge skip, same `allowed` tuple, same `sys.exit(1)` on the bad-list path (verify: file .github/workflows/ci.yml contains allowed = ("drafts/", "docs/workflow.md", ".agent/"))
- [ ] C5: The self-test takes its `allowed` tuple by parsing it out of the guard's own body rather than hard-coding a copy, so narrowing or widening the guard is followed instead of silently passed over (verify: file .github/workflows/ci.yml contains tuple(re.findall()
- [ ] C6: The diff of `.github/workflows/ci.yml` between fix base and head is a pure addition with zero deletions, touching only the `ci` job's checkout and the added self-test step. No `crates/` file, no `Cargo.toml` and no lockfile changes. Probed here on the pre-existing step's condition surviving; the zero-deletion scope is established by the validator from the compare API (verify: file .github/workflows/ci.yml contains if: github.event_name == 'push')
- [ ] C7: The pull-request run of both `ci` and `ci-windows` is green at the head SHA, and the `Cargo clippy` and `Cargo fmt check` steps are both green (verify: file .github/workflows/ci.yml contains Cargo clippy)
