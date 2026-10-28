# Requirements — Issue #1055 (the workflow checks in-game Settings and Help changes)

From Spec v1 (https://github.com/taftary/astrolith/issues/1055#issuecomment-6473057511), approved in https://github.com/taftary/astrolith/issues/1055#issuecomment-6473126898.

- [ ] C1: The specification skill lists what the Settings/Help review covers and how answers are recorded in the approved version (AC1) (verify: file .claude/skills/specification/SKILL.md contains Does it add, change, rename, move or remove an existing setting)
- [ ] C2: docs/workflow.md says the same and that answers sit on an `Answers:` line (AC1) (verify: file docs/workflow.md contains The answers sit on an `Answers:` line in each candidates section of the approved version)
- [ ] C3: The gate refuses missing sections, naming them (AC2) (verify: contains fixture impact-missing-sections: ok in python-tests.log)
- [ ] C4: The gate refuses a bare None (AC2) (verify: contains fixture impact-bare-none: ok in python-tests.log)
- [ ] C5: The gate refuses an unanswered candidate and accepts a fully answered one (AC2) (verify: contains fixture impact-unanswered: ok in python-tests.log)
- [ ] C6: The owner-test-only path is refused the same way (AC3) (verify: contains fixture impact-gate-refuses-owner-test-only approved: ok in python-tests.log)
- [ ] C7: The merge gate refuses Settings/Help code changed under a None spec and says it goes back to the owner (AC4) (verify: contains fixture touch-message-says-owner: ok in python-tests.log)
- [ ] C8: The merge gate accepts a change the spec lists, and tests-only changes (AC4) (verify: contains fixture touch-tests-only-accepted: ok in python-tests.log)
- [ ] C9: Specs approved before the change are not refused (AC5) (verify: contains fixture impact-gate-old-spec-kept: ok in python-tests.log)
- [ ] C10: The docs say plainly that the scripts check records, not meaning (AC6) (verify: file .claude/skills/specification/SKILL.md contains They check that the records and answers exist, not their meaning)
