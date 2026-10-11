"""Fixture tests for opencode_permissions.py against the real agent files.

Run: python scripts/agents/test_opencode_permissions.py
"""

import json
import pathlib
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import opencode_permissions as op  # noqa: E402

ROOT = HERE.parent.parent

CASES = [
    # (agent, tool, input, expected verdict)
    ("validator", "Bash", {"command": "cargo test --locked"}, "allow"),
    ("validator", "Bash", {"command": "git status --porcelain --untracked-files=no"}, "allow"),
    ("validator", "Bash", {"command": "git rev-parse HEAD"}, "allow"),
    ("validator", "Bash", {"command": "gh issue comment 5 --body '<!-- validator:pass sha=abc -->\n| a | b |; done'"}, "allow"),
    ("validator", "Bash", {"command": "git status && cargo test"}, "allow"),
    ("validator", "Bash", {"command": "git push origin main"}, "deny"),
    ("validator", "Bash", {"command": "git status && rm -rf crates"}, "deny"),
    ("validator", "Bash", {"command": "git status\nrm -rf crates"}, "deny"),
    ("validator", "Bash", {"command": "git log | sh"}, "deny"),
    ("validator", "Bash", {"command": "git show HEAD > out.txt"}, "deny"),
    ("validator", "Bash", {"command": 'gh issue comment 5 --body "$(rm -rf crates)"'}, "deny"),
    ("validator", "Bash", {"command": "gh issue comment 5 --body `rm -rf crates`"}, "deny"),
    ("validator", "Bash", {"command": "(rm -rf crates)"}, "deny"),
    ("validator", "PowerShell", {"command": "git status"}, "deny"),
    ("validator", "Edit", {"file_path": str(ROOT / "crates" / "x.rs")}, "deny"),
    ("validator", "Write", {"file_path": str(ROOT / "guides" / "x.md")}, "deny"),
    ("validator", "Read", {"file_path": str(ROOT / ".env")}, "deny"),
    ("validator", "Read", {"file_path": str(ROOT / "AGENTS.md")}, None),
    ("validator", "Agent", {"subagent_type": "Explore"}, "deny"),
    ("validator", "AskUserQuestion", {}, "deny"),
    ("guides", "Write", {"file_path": str(ROOT / "guides" / "topic" / "a.md")}, "allow"),
    ("guides", "Edit", {"file_path": str(ROOT / "crates" / "x.rs")}, "deny"),
    ("guides", "Edit", {"file_path": str(ROOT / ".opencode" / "agents" / "guides.md")}, "deny"),
    ("guides", "Bash", {"command": "git diff --stat"}, "allow"),
    ("guides", "Bash", {"command": "cargo test"}, "deny"),
    ("guides", "AskUserQuestion", {}, "allow"),
    ("main", "Agent", {"subagent_type": "validator"}, "allow"),
    ("main", "Agent", {"subagent_type": "Explore"}, "allow"),
    ("main", "Agent", {"subagent_type": "general-purpose"}, "deny"),
]


def run_hook(agent, tool, tool_input):
    proc = subprocess.run(
        [sys.executable, str(HERE / "opencode_permissions.py"), agent],
        input=json.dumps({"tool_name": tool, "tool_input": tool_input}),
        capture_output=True,
        text=True,
        cwd=ROOT,
    )
    if proc.returncode == 2:
        return "deny"
    if proc.returncode != 0:
        return f"error {proc.returncode}: {proc.stderr.strip()}"
    if proc.stdout.strip():
        return json.loads(proc.stdout)["hookSpecificOutput"]["permissionDecision"]
    return None


def main():
    failures = []
    for agent, tool, tool_input, expected in CASES:
        got = run_hook(agent, tool, tool_input)
        if got != expected:
            failures.append(f"{agent} {tool} {tool_input}: expected {expected}, got {got}")
    if run_hook("no-such-agent", "Bash", {"command": "ls"}) != "deny":
        failures.append("missing agent file must fail closed")
    if failures:
        print("opencode_permissions fixture failures:")
        for f in failures:
            print(f"  {f}")
        return 1
    print(f"opencode_permissions fixtures passed ({len(CASES) + 1} cases).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
