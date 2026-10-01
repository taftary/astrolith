# AI-Driven GitHub Workflow

## Purpose

The project needs a workflow that uses the GitHub repository as the central system for organizing all development work.

## Owner Needs

The owner needs to:

- Submit notions, feedback, and bugs.
- Answer clarification questions.
- Participate in planning decisions.
- Test the completed application.
- Confirm whether the original notion is correct.
- Report problems found during testing.

The owner does not need to:

- Write specifications.
- Create technical tasks.
- Write code.
- Write tests.
- Perform technical validation.
- Review pull requests.
- Manage workflow statuses.
- Inspect implementation details.

## AI Needs

The AI needs to:

- Create a GitHub Issue for every notion, feedback item, or bug.
- Ask clarification questions when the input is incomplete.
- Define the specification.
- Record the owner's answers and decisions.
- Create the implementation plan.
- Create and organize tasks.
- Implement the approved work.
- Create and execute tests.
- Perform technical validation.
- Perform visual validation when required.
- Create and manage pull requests.
- Record validation results.
- Correct technical problems.
- Present the completed application for owner testing.
- Record the owner's test feedback.
- Create correction work when the result is not correct.
- Complete the work when technical validation and owner testing are successful.

## GitHub Needs

The GitHub repository needs to contain and organize:

- Issues.
- Specifications.
- Plans.
- Tasks.
- Sub-issues.
- Labels.
- Projects.
- Milestones.
- Pull requests.
- Validation results.
- Owner feedback.
- Completion status.

All work must remain traceable from the original notion, feedback, or bug to the completed implementation.

## Workflow Needs

The workflow needs to follow this sequence:

1. Notion, feedback, or bug
2. GitHub Issue
3. Clarification
4. Specification
5. Plan
6. Tasks
7. Implementation
8. Technical validation
9. Pull request
10. Merge
11. Owner testing
12. Acceptance or correction

```text
Notion, feedback, or bug
        ↓
GitHub Issue
        ↓
Clarification
        ↓
Specification
        ↓
Plan
        ↓
Tasks
        ↓
Implementation
        ↓
Technical validation
        ↓
Pull request
        ↓
Merge
        ↓
Owner testing
        ↓
Acceptance or correction
```

## Completion Needs

Work is complete only when:

- The approved specification is implemented.
- Technical validation is successful.
- The pull request is completed.
- The implementation is merged.
- The owner tests the application.
- The owner confirms that the original notion is correct.

## Operating Principle

The owner provides the intention and tests the final result.
The AI manages the GitHub workflow, specification, planning, tasks, implementation, technical validation, pull request, and corrections.

## Notion Preservation

The AI must preserve the original notion.
The final implementation must be evaluated against the original input, not only against the technical specification.
The AI must ask before changing the notion.
If implementation reveals that the original idea is incomplete, contradictory, or technically impossible, the AI must stop and ask you.

## Owner Testing

Owner testing is product acceptance, not technical review.
You only test whether the application behaves as intended. You are not responsible for checking the code or technical evidence.

## Correction

Correction is part of the same workflow.
If your test shows that the result is wrong, the AI must create correction work and continue until the notion is accepted or explicitly abandoned.

## Project Board

Create one GitHub Project under the repo owner, link it to the repo, and turn on auto-add so every Issue (one per notion, feedback item, or bug) lands on its board.
The board needs only six statuses: Inbox, Needs your answer, Working, Ready to test, Needs correction, and Done.
The AI moves every item, and you only look at the board to see what's waiting on you, which is either a question to answer or something to test.
A merged PR moves the item to Ready to test, never to Done, so make sure the parent Issue isn't closed automatically by "Closes #123".
After you test, the AI marks it Done if it's right or Needs correction if it's not.
