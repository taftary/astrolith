# Requirements — Issue #1032 (validator verdicts can record hand-judged criteria)

From Spec v1 (https://github.com/taftary/astrolith/issues/1032#issuecomment-6435762817), approved in https://github.com/taftary/astrolith/issues/1032#issuecomment-6435766771.

- [ ] C1: The validator can record MET or NOT MET with a one-line reason for a check whose text says "judged by the validator", and the posted verdict shows it as judged with the reason (AC1, Q1 A) (verify: contains ok judged-row-shows-reason in python-tests.log)
- [ ] C2: Judged checks count in `Requirements: MET k/n`; with every check MET, a clean runtime and no drift the verdict reads PASS (AC2, Q1 A) (verify: contains ok judged-all-met-passes in python-tests.log)
- [ ] C3: One judged NOT MET keeps the verdict FAIL (AC2) (verify: contains ok judged-not-met-fails in python-tests.log)
- [ ] C4: A hand-judged check left unjudged keeps the verdict FAIL (AC2) (verify: contains ok unjudged-hand-row-keeps-fail in python-tests.log)
- [ ] C5: A judgment on a check the script verifies itself is refused with a clear message (AC3) (verify: contains ok script-row-refused in python-tests.log)
- [ ] C6: A replacement verdict for the same commit needs an owner decision: a comment on the same Issue written by the repository owner (AC4, Q2 A) (verify: contains ok non-owner-decision-refused in python-tests.log)
- [ ] C7: The validator's instructions describe how to judge a check and when to post a replacement (AC6) (verify: file .opencode/agents/validator.md contains --owner-decision <URL of the owner's decision comment on this Issue>)
