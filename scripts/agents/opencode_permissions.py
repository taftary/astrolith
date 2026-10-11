"""Claude Code PreToolUse hook that enforces an OpenCode agent's permissions.

Usage (from a .claude/agents/<name>.md frontmatter hook):
    python scripts/agents/opencode_permissions.py <agent-name>

The rule source is the `permissions:` list in `.opencode/agents/<agent>.md`,
preceded by the shared `permissions` in `.opencode/opencode.jsonc`. There is
no second copy of the rules, so the two harnesses cannot drift apart.

Evaluation mirrors OpenCode: the last matching rule wins; `*` matches any run
of characters; a pattern ending in ` *` also matches the bare command.

Shell commands are split on control operators (quote-aware) and every part
must be allowed. Anything this script cannot judge safely (command
substitution, redirection, subshells, PowerShell) is denied when the agent
denies shell by default. Fail closed: if the rules cannot be read, deny.

Output protocol: deny -> exit 2 with the reason on stderr; explicit allow ->
JSON permissionDecision "allow"; no matching rule -> exit 0 silently, so the
normal Claude Code permission flow applies.
"""

import json
import os
import pathlib
import re
import shlex
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent.parent

SHELL_TOOLS = {"Bash"}
EDIT_TOOLS = {"Edit", "Write", "MultiEdit", "NotebookEdit"}
READ_TOOLS = {"Read"}
SUBAGENT_TOOLS = {"Agent", "Task"}
QUESTION_TOOLS = {"AskUserQuestion"}
SEPARATORS = {"&&", "||", ";", "|", "&", "|&", ";;", "\n"}


class RuleError(Exception):
    pass


def _unquote(value):
    value = value.strip()
    if len(value) >= 2 and value[0] == value[-1] and value[0] in "\"'":
        return value[1:-1]
    return value


def parse_frontmatter_permissions(text):
    """Parse the fixed `permissions:` list shape used in .opencode/agents/*.md."""
    lines = text.splitlines()
    if not lines or lines[0].strip() != "---":
        raise RuleError("no frontmatter")
    try:
        end = next(i for i in range(1, len(lines)) if lines[i].strip() == "---")
    except StopIteration as exc:
        raise RuleError("unterminated frontmatter") from exc
    rules = []
    in_perms = False
    current = None
    for line in lines[1:end]:
        if not line.strip():
            continue
        if not line.startswith((" ", "-")):
            in_perms = line.split(":", 1)[0].strip() == "permissions"
            continue
        if not in_perms:
            continue
        m = re.match(r"^\s*(-\s+)?(action|resource|effect)\s*:\s*(.*)$", line)
        if not m:
            raise RuleError(f"unrecognised permissions line: {line!r}")
        if m.group(1):
            current = {}
            rules.append(current)
        if current is None:
            raise RuleError("permission field before list item")
        current[m.group(2)] = _unquote(m.group(3))
    for rule in rules:
        if set(rule) != {"action", "resource", "effect"}:
            raise RuleError(f"incomplete rule: {rule}")
        if rule["effect"] not in ("allow", "deny", "ask"):
            raise RuleError(f"unknown effect: {rule}")
    return rules


def load_rules(agent):
    if not re.fullmatch(r"[a-z0-9-]+", agent):
        raise RuleError(f"bad agent name: {agent!r}")
    rules = []
    shared = ROOT / ".opencode" / "opencode.jsonc"
    if shared.is_file():
        raw = "\n".join(
            l for l in shared.read_text(encoding="utf-8").splitlines()
            if not l.strip().startswith("//")
        )
        rules.extend(json.loads(raw).get("permissions", []))
    agent_file = ROOT / ".opencode" / "agents" / f"{agent}.md"
    if not agent_file.is_file():
        raise RuleError(f"missing {agent_file.relative_to(ROOT).as_posix()}")
    rules.extend(parse_frontmatter_permissions(agent_file.read_text(encoding="utf-8")))
    return rules


def wildcard_match(pattern, value):
    regex = "".join(
        ".*" if c == "*" else "." if c == "?" else re.escape(c) for c in pattern
    )
    if re.fullmatch(regex, value, re.DOTALL):
        return True
    return pattern.endswith(" *") and value == pattern[:-2]


def decide(rules, action, resource, case_insensitive=False):
    """Last matching rule wins. Returns 'allow', 'deny', 'ask' or None."""
    verdict = None
    for rule in rules:
        if rule["action"] != action:
            continue
        pat, val = rule["resource"], resource
        if case_insensitive:
            pat, val = pat.lower(), val.lower()
        if wildcard_match(pat, val):
            verdict = rule["effect"]
    return verdict


def unsafe_shell_construct(command):
    """Command substitution or backticks outside single quotes."""
    in_single = False
    in_double = False
    i = 0
    while i < len(command):
        c = command[i]
        if c == "\\" and not in_single:
            i += 2
            continue
        if c == "'" and not in_double:
            in_single = not in_single
        elif c == '"' and not in_single:
            in_double = not in_double
        elif not in_single and (c == "`" or command.startswith("$(", i)):
            return True
        i += 1
    return False


def split_shell(command):
    """Split into simple commands; None if the command cannot be judged."""
    if unsafe_shell_construct(command):
        return None
    lex = shlex.shlex(command, posix=True, punctuation_chars=";&|<>()\n")
    lex.whitespace = " \t\r"
    lex.whitespace_split = True
    try:
        tokens = list(lex)
    except ValueError:
        return None
    parts, current = [], []
    for tok in tokens:
        if tok in SEPARATORS:
            if current:
                parts.append(current)
            current = []
            continue
        if tok and set(tok) <= set(";&|<>()\n"):
            return None  # redirection, subshell, or odd operator
        current.append(tok)
    if current:
        parts.append(current)
    return [" ".join(p) for p in parts]


def rel_path(path):
    p = pathlib.Path(path)
    project = pathlib.Path(os.environ.get("CLAUDE_PROJECT_DIR", ROOT))
    try:
        p = p.resolve().relative_to(project.resolve())
    except (ValueError, OSError):
        return p.as_posix()
    return p.as_posix()


def evaluate(rules, tool_name, tool_input):
    """Returns (verdict, reason)."""
    if tool_name in SHELL_TOOLS or tool_name == "PowerShell":
        restricted = any(r["action"] == "shell" and r["effect"] == "deny" for r in rules)
        if tool_name == "PowerShell":
            if restricted:
                return "deny", "PowerShell is not available to this agent; use Bash"
            return None, ""
        parts = split_shell(str(tool_input.get("command", "")))
        if parts is None:
            if restricted:
                return "deny", (
                    "command uses substitution, redirection, or a subshell, "
                    "which this agent's shell allowlist cannot judge"
                )
            return None, ""
        verdicts = [decide(rules, "shell", part) for part in parts] or [None]
        for part, v in zip(parts, verdicts):
            if v == "deny":
                return "deny", f"shell command not allowed for this agent: {part}"
        if "ask" in verdicts or None in verdicts:
            return None, ""
        return "allow", "matches the agent's shell allowlist"
    if tool_name in EDIT_TOOLS:
        path = rel_path(tool_input.get("file_path") or tool_input.get("notebook_path") or "")
        return decide(rules, "edit", path), f"edit of {path}"
    if tool_name in READ_TOOLS:
        path = rel_path(tool_input.get("file_path", ""))
        v = decide(rules, "read", path)
        return ("deny", f"read of {path}") if v == "deny" else (None, "")
    if tool_name in SUBAGENT_TOOLS:
        target = str(tool_input.get("subagent_type") or "general-purpose")
        return decide(rules, "subagent", target, case_insensitive=True), f"subagent {target}"
    if tool_name in QUESTION_TOOLS:
        return decide(rules, "question", "*"), "asking the owner"
    return None, ""


def main(argv):
    if len(argv) != 2:
        print("usage: opencode_permissions.py <agent-name>", file=sys.stderr)
        return 2
    try:
        payload = json.load(sys.stdin)
        rules = load_rules(argv[1])
    except (RuleError, ValueError, OSError) as exc:
        print(f"permission hook failed closed: {exc}", file=sys.stderr)
        return 2
    verdict, reason = evaluate(
        rules, payload.get("tool_name", ""), payload.get("tool_input") or {}
    )
    if verdict == "deny":
        print(f"Blocked by .opencode/agents/{argv[1]}.md permissions: {reason}", file=sys.stderr)
        return 2
    if verdict == "allow":
        print(json.dumps({"hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "allow",
            "permissionDecisionReason": reason,
        }}))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
