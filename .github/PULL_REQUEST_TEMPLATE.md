Related to #N
<!-- Replace N with the parent Issue number. Keep this line: it links this PR to its parent without auto-closing it. -->

## Sub-issues

- [ ] #<sub-issue-number> — <done-criteria summary>
<!-- List every sub-issue this PR implements. Sub-issue PRs/descriptions may use auto-close keywords (e.g. `Closes #X`) for their own sub-issue only. -->

## Validator pass

- Verdict link: <link to the `Validator: PASS` issue comment>
- Commit SHA: <full head SHA the verdict covers, must match `validator:pass sha=`>

**Warning: never use Closes, Fixes, or Resolves (or any variant such as Close/Closed/Fix/Fixed/Resolve/Resolved) for the parent Issue. Merging this PR must not auto-close the parent — only sub-issues may use auto-close keywords.**

## Notes

<!-- Anything else the owner should know before testing. -->
