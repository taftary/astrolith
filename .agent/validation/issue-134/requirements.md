# Requirements (CONFIRMED) — issue #134

Derived by the main agent from approved Spec v1
(https://github.com/taftary/astrolith/issues/134#issuecomment-5977084063,
owner approval https://github.com/taftary/astrolith/issues/134#issuecomment-5977088691).
One criterion per acceptance criterion, in owner words. Criteria about
rendered pictures are judged by the validator with its own eyes
(body_html `<img`, HTTP round-trips); the script marks those
UNVERIFIABLE and they never count as MET from a log line.

- [ ] C1: verdict shows a before/after picture pair for every level whose frame differs from main, each labelled with the level name and the commit it comes from, plus a collapsed section holding all 11 after frames
- [ ] C2: every picture comes from the exact commit named in the verdict (same SHA as the validator marker); the before pictures come from main at validation time
- [ ] C3: with no visual change the verdict carries exactly one line stating the frames are identical to main, with the number of frames compared
- [ ] C4: a mismatch is a failure, never silent: spec says user-visible but no frame differs, or a frame differs but the spec names no user-visible change; the verdict names which one happened and the levels concerned
- [ ] C5: picture files live in the repository download area (hidden release entry for validation evidence) named with Issue number, commit, and level; none is committed to a branch; validating one commit twice overwrites instead of piling copies
- [ ] C6: the validator does the whole thing alone with existing access and tools (capture, convert, publish, embed); no owner action is needed per validation
- [ ] C7: the process documents say the same thing (Validator Environment, both skills, validator agent file, capabilities row) (verify: file docs/workflow.md contains frame_proof.py)
- [ ] C8: a picture step that fails (cannot capture, convert, or publish) turns the verdict to BLOCKED naming the missing piece, never to PASS
