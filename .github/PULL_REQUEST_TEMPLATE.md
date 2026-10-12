Related to #N
<!-- Replace N with the parent Issue number. Keep this line: it links this PR to its parent without auto-closing it. -->

## Sub-issues

- [x] #<sub-issue-number> — <done-criteria summary>
<!-- List every sub-issue this PR implements. Each was built on its own task branch and closed when `development.py merge-task` merged it into this branch, so no auto-close keyword is needed. -->

## Validator pass

- Verdict link: <link to the `Validator: PASS` issue comment>
- Commit SHA: <full head SHA the verdict covers, must match `validator:pass sha=`>

**Warning: never use Closes, Fixes, or Resolves (or any variant such as Close/Closed/Fix/Fixed/Resolve/Resolved) for the parent Issue. Merging this PR must not auto-close the parent — only sub-issues may use auto-close keywords.**

## Notes

<!-- Anything else the owner should know before testing. -->

## Rules

Does this change add, change, or retire a rule in `docs/engineering.md`? If yes, which ID and in which commit.
<!-- Every rule change lands in the same PR as its code, per docs/engineering.md. -->
