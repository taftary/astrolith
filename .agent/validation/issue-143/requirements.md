# Requirements (CONFIRMED) — issue #143

Derived by the main agent from approved Spec v1
(https://github.com/taftary/astrolith/issues/143#issuecomment-5979325478,
owner approval https://github.com/taftary/astrolith/issues/143#issuecomment-5979464043).
One criterion per acceptance criterion, in owner words. Criteria about
reading sources on the web, the hardware comment on the Issue, and the
pull request's file list are judged by the validator directly; the
script marks those UNVERIFIABLE and they never count as MET from a log
line.

- [ ] C1: `docs/universe/realism-review.md` exists on main after the merge and contains the twelve sections in order with the stated headings (verify: file docs/universe/realism-review.md contains ## 12. Sources)
- [ ] C2: every level L1-L11 appears in the current-state audit and in the realism-targets table, each row with a sourced target value and today's value (verify: file docs/universe/realism-review.md contains ## 4. Realism targets per level)
- [ ] C3: each of the six profiles has its own findings section with at least three findings, every finding carrying a source number and a recommendation (verify: file docs/universe/realism-review.md contains Planetary scientist)
- [ ] C4: each of the seven themes has the problem, the realism target, two or more options with trade-offs, the recommended option, the rules it touches, its dependencies, a size estimate, and a follow-up Issue title (verify: file docs/universe/realism-review.md contains Follow-up Issue title)
- [ ] C5: the proposed-ladder table shows current and proposed side by side, every changed row says "needs owner approval", and a planetary system is a star with planets, not stars inside a star (verify: file docs/universe/realism-review.md contains needs owner approval)
- [ ] C6: the loading and traversal sketch states the pass-through rule in the owner's words (the target always continues; what is passed through shows its children), describes a free-flight mode and how to switch, and gives a concrete replacement bound for the live-cell limit with a hard cap (verify: file docs/universe/realism-review.md contains hard cap)
- [ ] C7: the implementation order covers all seven themes with a reason for each position and puts geometry before statistics before home path before appearance before dynamics wherever themes are independent (verify: file docs/universe/realism-review.md contains ## 10. Implementation order)
- [ ] C8: every source has a title, a public address, and a read date; at least twenty sources; a sample of at least five addresses chosen by the validator loads and supports the claim it is cited for
- [ ] C9: the hardware baseline is in the document and in a comment on the Issue, with no user name, host name, serial number, or credential (verify: file docs/universe/realism-review.md contains ## 11. Hardware baseline)
- [ ] C10: nothing in the application changed: --verify still ends in VERIFY-OK, the golden files are untouched, and the pull request touches only documentation plus the review checker and this requirements file (verify: contains VERIFY-OK in verify.log)
- [ ] C11: `python scripts/validation/review_lint.py` prints REVIEW-LINT-OK when the document has the twelve headings in order, every finding and targets row cites an existing source number, every source has an address and a date, and there are at least twenty sources; otherwise a REVIEW-LINT-FAIL line naming the first problem (verify: file scripts/validation/review_lint.py contains REVIEW-LINT-OK)
- [ ] C12: the document is linked from docs/README.md and from docs/universe/ladder.md with a one-line pointer saying proposed ladder changes live in the review and are not yet approved (verify: file docs/universe/ladder.md contains realism-review.md)
