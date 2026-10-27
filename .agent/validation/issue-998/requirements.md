# Requirements — Issue #998 (continuous scaling for L1-L7, the study)

From Spec v1 (https://github.com/taftary/astrolith/issues/998#issuecomment-6440169359), approved in https://github.com/taftary/astrolith/issues/998#issuecomment-6440245588.

- [ ] C1: A headless prototype zooms continuously from L1 to L7 and back along the journey and passes straight through a marker in free flight, measuring every handover's position, direction and speed jumps (AC1, Q4 A) (verify: contains the_journey_hands_over_exactly_through_l1_to_l7_and_back ... ok in cargo-test.log)
- [ ] C2: The free-flight pass opens and closes the marker and stays on its straight line (AC1, Q4 A) (verify: contains a_free_pass_through_a_marker_stays_on_its_line ... ok in cargo-test.log)
- [ ] C3: The coordinate step at the camera is measured at every scale L1-L7 (AC2) (verify: file .agent/validation/issue-998/study.txt contains SCALE-PRECISION way=in level=L6)
- [ ] C4: Live markers and cells along the journey are counted against a budget of 32 cells and 2,000 points, by size on screen (AC3, Q3 A) (verify: contains the_crowds_stay_inside_the_budget_along_the_journey ... ok in cargo-test.log)
- [ ] C5: The per-frame streaming and selection work is measured on this PC, one thread (AC4) (verify: file .agent/validation/issue-998/study.txt contains SCALE-COST level=L6)
- [ ] C6: A go / no-go recommendation with the numbers, the risks and a draft of the build phases is posted, and no visible work starts before the owner's go (AC5) (judged by the validator from the recommendation comment on #998 that this PR's notes link, against `.agent/validation/issue-998/study.txt`; no script probe)
- [ ] C7: Nothing visible changes yet: the headless game check passes with the same results (AC6) (verify: contains VERIFY-OK in verify.log)
