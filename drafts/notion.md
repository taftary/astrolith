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

## AI Operating Model

The workflow uses instructions, skills, and two agents. All of them are written in the repository, so the work never depends on what an agent remembers between sessions.

### Instructions

Short rules that every agent always follows:

- Preserve the original notion. Ask the owner before changing it.
- Stop and ask the owner if the notion is incomplete, contradictory, or technically impossible.
- A merged pull request moves the item to "Ready to test", never to "Done". The parent Issue is not closed automatically.
- An item cannot move to "Ready to test" without a validator pass.
- Credentials and secrets are never written in Issues or comments.

### Skills

Step-by-step procedures loaded only when a stage needs them, one per stage: clarification, specification (including the test plan), planning, tasks, implementation, technical validation, visual validation, pull request, correction, and presenting for owner testing.

### Agents

#### Main Agent

Owns the workflow from intake to completion. It:

- Talks to the owner.
- Writes the specification, plan, and tasks.
- Implements and corrects.
- Opens pull requests and moves items on the Project board.

#### Validator Agent

Independently checks the work. It:

- Never writes or edits code.
- Receives only the original notion (verbatim), the approved specification, and the result. It does not receive the main agent's reasoning.
- Checks that the result matches the specification, and that the specification and result still match the original notion.
- Records a pass or fail with reasons and evidence on the Issue.
- If the validator finds that the specification has drifted from the original notion, the main agent must stop and ask the owner. On a fail, the item returns to the main agent for correction.

### Validator Environment

The validator tests the real application in an isolated test environment.
It interacts through the same interface a human uses: it sees the screen, clicks, types, and navigates.
It does not use internal APIs, the database, or the code to perform the behavior being tested. These are allowed only to prepare a starting state.
It can start from a known state (test accounts, seed data, reset).
It can read errors and logs.
It records evidence (screenshots and steps taken) on the Issue.
Aspects that need human judgment, such as feel, timing, and overall quality, are left to owner testing.

### Testability Check

For every notion, before the plan is created, the main agent must confirm that the validator can test the result the way a human would.
The specification includes a test plan: the steps a human would take to verify the notion, and what they should see.
The main agent lists what the validator needs to run that plan: access to the test environment, test accounts, starting data, and any third-party services.
The main agent asks the owner only for what the owner alone can provide, in non-technical terms. The question is posted on the Issue and the item moves to "Needs your answer".
The validator runs a preflight: it reaches the starting state and performs the first step of the test plan. The main agent's statement that access exists is not enough.
Implementation does not start until the preflight passes.
If the notion cannot be tested by the validator, the main agent must:

- Build the missing capability as a separate work item first, or
- Obtain the missing access from the owner, or
- With the owner's explicit approval, mark the notion as owner-test-only, recorded on the Issue.

A notion is never left untested silently. Capabilities already proven are recorded once in the repository, and later notions check only what is new.

### Sync and Governance

This workflow document is the source of truth for the process. Instructions and skills implement it and must not contradict it.
When the process changes, this document is changed first. Instructions and skills are updated in the same pull request.
Rules that protect the owner (notion preservation, the validator block, the testability check, secrets handling) can be changed only with the owner's approval.
The AI may propose changes to skills freely. Changes to the instructions above require the owner's approval.
The validator's tools are not modified in the same work item they are used to validate. Changes to them are recorded separately.
