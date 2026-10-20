# Requirements — Issue #847 (faster screenshots, tests and validator)

From Spec v1 (https://github.com/taftary/astrolith/issues/847#issuecomment-6290353473), approved in https://github.com/taftary/astrolith/issues/847#issuecomment-6290362082.

- [ ] C1: Screenshot mode can run chosen shot groups or a smoke set, and runs everything by default (AC1) (verify: contains screens_run_the_chosen_groups ... ok in cargo-test.log)
- [ ] C2: A written rule says which groups a change must run (AC2) (verify: file .claude/skills/visual-validation/SKILL.md contains Shot groups)
- [ ] C3: The slowest tests run faster, check the same things, and the before and after times are recorded (AC3) (verify: file docs/decisions/0043-faster-checks.md contains Before)
- [ ] C4: The default local test command runs the fast set; CI and the validator run everything (AC4) (verify: file .github/workflows/ci.yml contains include-ignored)
- [ ] C5: The validation script runs the headless check once plus one repeat, and takes test results from CI's passed run on the same commit (AC5) (verify: contains test_validate_reuses_ci_results in python-tests.log)
- [ ] C6: The measured times are recorded, with the goals of under 180 s and under 2 min (AC6) (verify: file docs/decisions/0043-faster-checks.md contains 180 s)
- [ ] C7: The workflow docs and skills match the new behaviour (AC7) (verify: file docs/workflow.md contains 0043)
- [ ] C8: The game behaves as before, and the headless game check passes (AC8) (verify: contains VERIFY-OK in verify.log)
