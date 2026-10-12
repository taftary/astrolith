# Requirements (CONFIRMED) - issue #533

Derived by the main agent from approved Spec v1
(https://github.com/taftary/astrolith/issues/533#issuecomment-6118342710,
owner approval https://github.com/taftary/astrolith/issues/533#issuecomment-6118369098).
One criterion per acceptance criterion, in the spec's words. Probes read the
repo's Python fixture tests (python-tests.log) and files at the head commit.
The run time against the baseline (about 9 min, 48 steps) is reported in
the validation summary (test plan step 4); the merged result (parent still
open, sub-issues closed) is checked in the owner test.

- [ ] C1: When a PR for a parent issue is merged, the parent issue stays open: branches are never linked through the Development sidebar (verify: contains ok parent-branch-no-development-link in python-tests.log)
- [ ] C2: A requirements line numbered R (or any other letter) is checked and counted in MET n/n exactly like a C line (verify: contains ok every-prefix-counted in python-tests.log)
- [ ] C3: Sub-issues never appear on the board, and each one is closed when its branch is merged into the parent branch (verify: contains ok merge-task-closes-naming-commit in python-tests.log)
- [ ] C4: Each sub-issue has its own branch made from the parent branch and merged into it with no PR; only the parent branch has a PR to main (verify: contains ok merge-task-no-pr in python-tests.log)
- [ ] C5: The validator's verdict comment is produced by the script in the exact form the merge check reads (verify: contains ok verdict-passes-merge-gate in python-tests.log)
- [ ] C6: The validator takes no screenshots and uploads no pictures, the script makes no frame pictures, and the merge check passes without a Visual line (verify: contains fixture pass-without-visual: ok in python-tests.log)
- [ ] C7: A validator run needs far fewer steps: every input comes in one call (verify: file .opencode/agents/validator.md contains scripts/validation/fetch_inputs.py)
- [ ] C8: The workflow docs and skills describe the new flow (verify: file docs/workflow.md contains development.py merge-task)
