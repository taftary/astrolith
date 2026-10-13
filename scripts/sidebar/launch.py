"""Starts an AI on an issue in its own worktree (#613, P2 A).

  python scripts/sidebar/launch.py --issue N --agent claude|opencode [--slug S] [--dry-run]
  python scripts/sidebar/launch.py --list

The issue must be set up for a worktree (the owner chose it; see
`development.py choose`). When its worktree does not exist yet, `--slug`
creates it through `development.py create-branch`. The agent starts inside
the worktree with the issue's prompt, detached, and writes to the main
checkout's `.agent/agents/<N>.log`. Running agents are recorded in
`.agent/agents/running.json`; a second agent on an issue whose agent is
still running is refused. `--list` shows them. `--dry-run` prints the
command and starts nothing (the fixture tests use it).

Agent commands come from `AGENT_COMMANDS`, overridable through
`.agent/agents/commands.json` ({"claude": ["claude", "-p", "{prompt}"]}).
JSON to stdout.
"""
import argparse
import datetime
import json
import os
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import development  # noqa: E402
import worktrees  # noqa: E402

AGENT_COMMANDS = {
    "claude": ["claude", "-p", "{prompt}", "--permission-mode", "acceptEdits"],
    "opencode": ["opencode", "run", "{prompt}"],
}

PROMPT = (
    "You work on issue #{issue} of this repository, in this worktree, following AGENTS.md "
    "and the workflow skills. The owner chose a worktree for this issue. Every message you "
    "send the owner starts with '#{issue}:' and goes through "
    "`python scripts/sidebar/notify.py --issue {issue} \"...\"`. The owner's answers arrive "
    "as comments on issue #{issue}; read them there. Never work outside this worktree."
)


def agents_dir(main):
    return worktrees.evidence_root(main) / "agents"


def registry(main):
    path = agents_dir(main) / "running.json"
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return {}


def save_registry(main, data):
    d = agents_dir(main)
    d.mkdir(parents=True, exist_ok=True)
    (d / "running.json").write_text(json.dumps(data, indent=2), encoding="utf-8")


def alive(pid):
    """Whether process `pid` still runs; tests replace this."""
    if not pid:
        return False
    if sys.platform.startswith("win"):
        p = subprocess.run(["tasklist", "/FI", f"PID eq {pid}", "/NH"],
                           capture_output=True, text=True, errors="replace")
        return str(pid) in (p.stdout or "")
    try:
        os.kill(int(pid), 0)
        return True
    except OSError:
        return False


def commands(main):
    """Agent commands, with the owner's overrides."""
    out = dict(AGENT_COMMANDS)
    try:
        out.update(json.loads((agents_dir(main) / "commands.json").read_text(encoding="utf-8")))
    except (OSError, ValueError):
        pass
    return out


def start(cmd, cwd, log):
    """Starts `cmd` detached in `cwd`, writing to `log`; returns its pid.
    Tests replace this."""
    log.parent.mkdir(parents=True, exist_ok=True)
    handle = open(log, "a", encoding="utf-8")
    flags = 0
    if sys.platform.startswith("win"):
        flags = subprocess.DETACHED_PROCESS | subprocess.CREATE_NEW_PROCESS_GROUP
    proc = subprocess.Popen(cmd, cwd=str(cwd), stdout=handle, stderr=subprocess.STDOUT,
                            stdin=subprocess.DEVNULL, creationflags=flags)
    return proc.pid


def cmd_list(main):
    rows = []
    for issue, entry in sorted(registry(main).items(), key=lambda kv: int(kv[0])):
        rows.append({**entry, "issue": int(issue), "running": alive(entry.get("pid"))})
    print(json.dumps({"ok": True, "agents": rows}))
    return 0


def main(argv=None):
    ap = argparse.ArgumentParser(description="Start an AI on an issue in its worktree")
    ap.add_argument("--issue", type=int)
    ap.add_argument("--agent", choices=sorted(AGENT_COMMANDS))
    ap.add_argument("--slug", default="")
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--list", action="store_true")
    a = ap.parse_args(argv)
    main_dir = development.main_checkout()
    if a.list:
        return cmd_list(main_dir)
    if a.issue is None or a.agent is None:
        print(json.dumps({"ok": False, "error": "--issue and --agent are required"}))
        return 1
    running = registry(main_dir).get(str(a.issue))
    if running and alive(running.get("pid")):
        print(json.dumps({"ok": False, "issue": a.issue, "error":
                          f"#{a.issue} already has a running {running.get('agent')} agent "
                          f"(pid {running.get('pid')})"}))
        return 1
    if development.way_of(a.issue) != "worktree":
        print(json.dumps({"ok": False, "issue": a.issue, "error":
                          "the owner has not chosen a worktree for this issue "
                          "(development.py choose --mode worktree)"}))
        return 1
    folder = development.issue_worktree(main_dir, a.issue)
    if folder is None:
        if not a.slug:
            print(json.dumps({"ok": False, "issue": a.issue,
                              "error": "no worktree yet: pass --slug to create it"}))
            return 1
        if not a.dry_run and development.main(
                ["create-branch", "--issue", str(a.issue), "--slug", a.slug]) != 0:
            return 1
        folder = worktrees.issue_folder(main_dir, a.issue, a.slug)
    prompt = PROMPT.format(issue=a.issue)
    cmd = [part.replace("{prompt}", prompt) for part in commands(main_dir)[a.agent]]
    log = agents_dir(main_dir) / f"{a.issue}.log"
    if a.dry_run:
        print(json.dumps({"ok": True, "dryRun": True, "issue": a.issue, "agent": a.agent,
                          "cwd": str(folder), "command": cmd, "log": str(log)}))
        return 0
    pid = start(cmd, folder, log)
    data = registry(main_dir)
    data[str(a.issue)] = {"agent": a.agent, "pid": pid, "worktree": str(folder),
                          "log": str(log),
                          "started": datetime.datetime.now(datetime.timezone.utc).isoformat()}
    save_registry(main_dir, data)
    print(json.dumps({"ok": True, "issue": a.issue, "agent": a.agent, "pid": pid,
                      "cwd": str(folder), "log": str(log)}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
