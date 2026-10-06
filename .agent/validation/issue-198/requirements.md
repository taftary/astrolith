# Requirements (CONFIRMED) — issue #198

Derived by the main agent from approved Spec v3
(https://github.com/taftary/astrolith/issues/198#issuecomment-6009698760,
owner approval https://github.com/taftary/astrolith/issues/198#issuecomment-6009708225).
This version voids the Spec v1 derivation below it in history: C1 is the
worst case, not the warm case, and test plan step 1 uses the stated split
of labour (main agent performs transitions, validator verifies).
One criterion per acceptance criterion, plus one per owner-expectation sentence
from the notion, in owner words. Owner approval of the spec is confirmation of
this criteria text.

Timings, run counts, tree comparisons, and cache locations are judged by the
validator from the elapsed-time lines, run records, and fixture output that this
notion's own tooling writes. The script marks those UNVERIFIABLE; they never
count as MET from a summary claim, and they never count against the run
observations that `merge_gate.py` owns.

- [ ] C1: The worst single validation run takes under 5 minutes on the dev PC, and the run's own record states how long it took; the single named exception is the first run after a cache directory has been wiped, which is measured, recorded, not counted, and must not happen twice
- [ ] C2: Between opening a pull request and handing the result to the owner, the full validation script runs exactly once; the main agent still runs the test suite and the headless app run first, so a broken build is caught early and cheaply (verify: contains VERIFY-OK in verify.log)
- [ ] C3: After a merge, validation on main compares the contents of the merged commit against the validated commit and records that comparison; when they are identical it runs the app once, and when they differ the full validation runs instead (verify: contains VERIFY-OK in verify.log)
- [ ] C4: A picture proof blocked by an infrastructure cause may let the round merge with the reason named, but only when all four conditions hold: the cause is named, the change touches no file that can alter a captured picture, the golden tests pass byte for byte, and the record names all three; when any one fails the full validation is required as before (verify: file scripts/gates/merge_gate.py contains BLOCKED (infra)
- [ ] C5: Filling in a pull request description with the validator's verdict no longer triggers a new CI run, and the deliberate close-and-reopen of the pull request to force one is gone; the comparison that the verdict covers the head commit still refuses to merge when the verdict names a different commit (verify: file .github/workflows/ci.yml contains no-close-reopen)
- [ ] C6: A picture proof reuses the previous proof's base-side pictures when the base commit has not moved, and prints the base commit they came from; when the base has moved, both sides are captured fresh (verify: file scripts/validation/frame_proof.py contains base-reused)
- [ ] C7: The two build caches sit in the repository's ignored cache folder rather than in the operating system's temporary folder, and each capture's log records the source commit, whether that tree was clean, and the build's fingerprint (verify: contains identity head= in capture-after.log)
- [ ] C8: The testability preflight runs only when something is actually unknown: it does not run when every testing need in the approved specification already has a proven entry in docs/validator-capabilities.md, and it runs exactly as today when any need is missing, unproven, or the capabilities file has not been reviewed; a skip names the capabilities file and the specific entries, and either way the fact is recorded on the Issue
- [ ] C9: Planning and task creation are one step producing one record that holds both the plan and the task list with its done criteria, instead of two steps and two records; the content itself does not change (verify: file docs/workflow.md contains plan-and-tasks)
- [ ] C10: For one round, the time spent validating is less than the time spent implementing; on issue #157 it was the reverse, 3.2 hours of validation against 1.5 hours of implementation
- [ ] C11: The process lifecycle should not be made really slow by the number of validation processes it has (verify: contains VERIFY-OK in verify.log)
- [ ] C12: The maximum cost of a validation round is reduced, so the slowest round no longer costs what issue #157 cost
