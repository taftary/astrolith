# Requirements (CONFIRMED) — issue #287

Derived by the main agent from approved Spec v1
(https://github.com/taftary/astrolith/issues/287#issuecomment-6041393226,
owner approval recorded on the Issue).
One criterion per acceptance criterion, plus one per owner-expectation sentence
from the notion, in owner words. Owner approval of the spec is confirmation of
this criteria text.

- [ ] C1: a failed picture upload writes the complete error reason in publish.log, not a fragment (verify: file scripts/validation/frame_proof.py contains full-errors-logged)
- [ ] C2: a failed upload is retried a few times with waits before the run gives up (verify: file scripts/validation/frame_proof.py contains retry-with-backoff)
- [ ] C3: before uploading, the run deletes that issue's assets from older commits, keeping only the newest commit's pictures (verify: file scripts/validation/frame_proof.py contains prune-superseded)
- [ ] C4: the release drops back below full as part of this work so the #225 gate can re-run (verify: release validation-evidence lists fewer than 1000 assets)
- [ ] C5: nothing else in the repo changes meaningfully, the frozen ladder table stays unchanged (verify: file docs/universes/ladder.md contains Scale ladder)
