# Requirements — issue #88 (CONFIRMED)

Derived by the specification skill from approved Spec v2
(https://github.com/taftary/astrolith/issues/88#issuecomment-5961669552,
approval: https://github.com/taftary/astrolith/issues/88#issuecomment-5961680682).
Spec v1 (https://github.com/taftary/astrolith/issues/88#issuecomment-5957392146) is
withdrawn, not amended: its criteria could not be discharged by the tool that
grades them.
Owner approval of the spec confirms the criteria text below. Process-layer
criteria use file probes (worktree reads); runtime-layer criteria use log
probes. Probes are the AI's addition; the owner never sees this syntax.

## Why this file has nine criteria of exactly one fact each

Three gates have to agree, and reading all three is what forced the rewrite:

- `validate.py:200` and `:214` accept one probe per criterion, in one of two
  forms, both patterns anchored to the end of the criterion string.
- `validate.py:271` returns PASS only when every row is MET. One UNVERIFIABLE or
  NOT MET row makes the whole run FAIL. An unparsable probe becomes UNVERIFIABLE
  at `:235`.
- `merge_gate.py:156-158` counts MET rows in `validate.py`'s report and requires
  that count to equal the verdict's `MET n/n`.

Together: every criterion must be exactly one positive fact, with exactly one
probe, resolving to MET. Spec v1's criteria file, run through those two regexes
with no runtime steps at all, predicted MET 0/5 and verdict FAIL - a function of
probe syntax alone, which no correct implementation could have changed.

Consequence for the reader: a multi-fact criterion here would silently collapse.
Spec v1's C1 had three semicolon-separated probes, which the end-anchored regex
joined into one 120-character needle that is not in the file, yielding NOT MET.

## What these probes do and do not prove

Every criterion below is a file or runtime-log probe. Together they establish
that the mechanism is present and unaltered in the shipped tree. They do **not**
establish that the harness permitted the three newly allowed commands. No file
probe can observe a sandbox permission decision, so that fact is discharged by
the validator's own recorded command run in its verdict comment, which is the
record the merge gate reads alongside this report. Stated plainly so a green row
here is never mistaken for a proof of permission.

Two criteria carry a named validator-recorded half, marked inline below:

- **AC4's negative half** — that no bare `cargo *` or `git *` entry exists, and
  that the three new entries come *after* the broad shell deny. The grammar has
  no negation form, and string presence does not establish ordering, so the
  validator parses the frontmatter and reports each entry's index.
- **AC9's test-suite half** — structurally covered rather than merely recorded:
  `validate.py` appends to `errors` on any test failure, non-zero exit, exception
  or panic across all three of its logs, and any error forces the verdict to FAIL
  at `:274`. The grader enforces it, not a criterion.

## What is deliberately not a graded criterion

Spec v1's C3 asked whether the validator can actually run `cargo doc`, `cargo fmt`
and `cargo deny`. That is the capability this Issue exists to deliver, and it
cannot be a MET row: no file probe observes a permission decision and none of
`validate.py`'s three logs contains their output. Spec v2 records this as a real
loss of graded coverage rather than dressing it up. The fact is carried instead
by the validator's preflight run on this Issue, recorded in its verdict comment,
and end to end by Issue #85's post-merge preflight re-run, which is where the
capability was originally needed.

Expected there: `cargo doc` and `cargo fmt --check` succeed, and **`cargo deny`
runs and then fails on a missing `deny.toml`**, which arrives in M2 as the bug
text itself records. A permission denial and a command failing on absent
configuration are different facts; only the first indicates a problem.

## Layer 1 criteria (AC1-AC9)

- [ ] C1: `.opencode/agents/validator.md` grants `cargo doc` to the validator with an explicit `effect: allow` entry (verify: file .opencode/agents/validator.md contains resource: "cargo doc *")
- [ ] C2: `.opencode/agents/validator.md` grants `cargo fmt` to the validator the same way (verify: file .opencode/agents/validator.md contains resource: "cargo fmt *")
- [ ] C3: `.opencode/agents/validator.md` grants `cargo deny` to the validator the same way (verify: file .opencode/agents/validator.md contains resource: "cargo deny *")
- [ ] C4: Deny rules are still present in `.opencode/agents/validator.md`, so narrow-by-default survives the widening. Probe anchors that deny rules exist; the load-bearing negative, that no bare `cargo *` or `git *` entry exists and that the three new entries come after the broad shell deny, is validator-recorded because the grammar has no negation form and string presence does not establish ordering (verify: file .opencode/agents/validator.md contains effect: deny)
- [ ] C5: `scripts/validation/compare_files.py` exists and its `--help` documents a hash mode (verify: file scripts/validation/compare_files.py contains hash PATH)
- [ ] C6: The same script's `--help` documents a compare mode (verify: file scripts/validation/compare_files.py contains compare LEFT RIGHT)
- [ ] C7: The script reports a located difference rather than a bare "they differ", because a validator must be able to point at the divergence (verify: file scripts/validation/compare_files.py contains DIFFER at)
- [ ] C8: The script reports the identical outcome with a file count (verify: file scripts/validation/compare_files.py contains identical (%d file(s) compared))
- [ ] C9: The headless application check still runs clean at the head SHA, so this Issue regresses nothing. Probe covers `VERIFY-OK`; the test-suite half is enforced by `validate.py`'s own error scan, which forces FAIL at `:274` on any failure, non-zero exit, exception or panic (verify: contains VERIFY-OK in verify.log)
