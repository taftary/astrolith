# Requirements (CONFIRMED) - issue #1235

Original bug report: https://github.com/taftary/astrolith/issues/1235

Approved Spec v1: https://github.com/taftary/astrolith/issues/1235#issuecomment-6694641496

Owner approval: https://github.com/taftary/astrolith/issues/1235#issuecomment-6694764788

The four criteria below preserve the approved acceptance criteria. Compound
behavior is judged from executed fixture evidence and the public verdict path,
not inferred from a source-text match. Runtime and exact-commit merge checks
remain separate gates, not artificial additional acceptance criteria.

- [ ] C1: For a confirmed, nonempty checklist assessed entirely by the validator, successful runtime checks plus an explicit, evidenced MET judgment for every criterion can produce a consistent successful verdict. The final result does not retain a false runtime blocker merely because no criterion had an automated probe. (judged by the validator from test_entirely_judged_checklist_passes_after_all_met in scripts/validation/test_judged_verdict.py, its executed python-tests.log output, and the real generated verdict)
- [ ] C2: Missing or negative judgments never produce success. A checklist mixing automated checks and validator judgments continues to work, and a manual judgment cannot replace or override a criterion checked by the script. (judged by the validator from the missing/negative, mixed, invalid, unmet/unavailable automated-probe fixtures in scripts/validation/test_judged_verdict.py and their executed python-tests.log output)
- [ ] C3: Genuine blockers and failures remain non-successful: an empty or unconfirmed checklist, failed runtime check, wrong commit, dirty tracked tree, unmet automated criterion or unapproved drift cannot be cleared by supplying positive judgments. (judged by the validator from the blocker/runtime/automation/drift fixtures in scripts/validation/test_judged_verdict.py, their executed python-tests.log output, and existing gate fixtures)
- [ ] C4: The posted verdict, its machine-readable marker, runtime status and requirement count agree. The result still requires the existing exact-commit validator and merge gates; no manual verdict editing or artificial extra requirement is needed to make a result pass. (judged by the validator from the generated-output/posting fixtures in scripts/validation/test_judged_verdict.py, existing gate/compare fixtures, their executed python-tests.log output, and the actual public posting command at this PR head)

## Test-plan boundary

The generated-entrypoint fixtures are hermetic unit tests: cargo/git/CI/GitHub
responses are fixed inputs while the validation, real temporary evidence files,
judgment application and posting preparation run unchanged. They are not a
substitute for the independent validator's real runtime checks or live Issue
publication. The existing #1227 blocked verdict must remain intact; #1227 needs
a separate fresh exact-head validation after this repair is integrated.
