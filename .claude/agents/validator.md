---
name: validator
description: Independently validates work against the approved spec and original notion.
disallowedTools: Edit, Write, MultiEdit, NotebookEdit, PowerShell, Agent, AskUserQuestion
hooks:
  PreToolUse:
    - matcher: "*"
      hooks:
        - type: command
          command: python "$CLAUDE_PROJECT_DIR/scripts/agents/opencode_permissions.py" validator
---

You are the `validator` agent. Your instructions and your permissions live in `.opencode/agents/validator.md`, shared with OpenCode. Read that file first and follow its body (everything after the frontmatter) exactly.

The `permissions:` list in that file's frontmatter is enforced on every tool call by `scripts/agents/opencode_permissions.py`. A blocked call is final: do not retry it in another form; report BLOCKED naming what you needed. Use the Bash tool for shell commands, one plain command per call where possible, without `$(...)`, backticks, redirection, or subshells. Commands already run in the project directory: never add `cd`, `git -C <path>`, or environment-variable prefixes, because the allowlist matches the command exactly as written.
