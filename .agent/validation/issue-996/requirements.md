# Requirements — Issue #996 (marker spacing study, L1-L7)

From Spec v1 (https://github.com/taftary/astrolith/issues/996#issuecomment-6552659845), approved in https://github.com/taftary/astrolith/issues/996#issuecomment-6552665649.

- [ ] C1: The page has one row for each level L1 to L7 with the marker kind, the game gap, the real gap, the ratio, and the game count against the real count (AC1) (verify: file guides/universe/documents/marker-spacing.md contains ## 3. Game against real)
- [ ] C2: Every real number names its source, listed in the page's sources section (AC2) (verify: file guides/universe/documents/marker-spacing.md contains ### Sources)
- [ ] C3: Every game number is measured from the game at the fixed seed, and the page says how, so it can be measured again (AC3) (verify: file guides/universe/documents/marker-spacing.md contains ### Measuring script)
- [ ] C4: Each level has a "what fixing it would change" note and a recommendation: fix, fix partly, or leave (AC4) (verify: file guides/universe/documents/marker-spacing.md contains ## 5. Recommendation per level)
- [ ] C5: The game is unchanged: the pull request touches only guides/ files and this requirements record (AC5) (judged by the validator from the PR file list; no script probe)
- [ ] C6: A summary comment with the table and the question "which levels to fix?" is posted on the Issue (AC6) (judged by the validator from the Issue comments; no script probe)
- [ ] C7: Review the distance between markers and make them as realistic as possible: the study shows how far each level is from real and what would make it real (owner notion) (judged by the validator against the page; no script probe)
- [ ] C8: The headless check still passes (game unchanged) (verify: contains VERIFY-OK in verify.log)
