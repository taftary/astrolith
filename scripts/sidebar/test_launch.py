"""Fixture harness for the launcher and agent messages (issue #613, T4).

The git/gh runner, the process start and the liveness check are fakes, and
the main checkout is a temporary folder, so nothing starts, nothing touches
the network or the real repo (E-TEST-HERMETIC). Pins:
  - --dry-run prints the agent command inside the issue's worktree with the
    issue's prompt, and starts nothing;
  - a real launch records the agent, and --list shows it running;
  - a second agent on the same issue is refused while the first runs, and
    allowed once it has stopped;
  - an issue without the owner's worktree choice is refused;
  - agent messages are tagged `#N:` once, posted on the issue, and sent to
    Telegram only when the bot token and chat are set.

Usage:
  python scripts/sidebar/test_launch.py
Exit 0 when every fixture behaves; non-zero naming the first failure.
"""
import contextlib
import io
import json
import os
import sys
import tempfile
from pathlib import Path
from types import SimpleNamespace

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import development  # noqa: E402
import launch  # noqa: E402
import notify  # noqa: E402

YES = json.dumps(["<!-- worktree:yes -->"])
NO = json.dumps(["<!-- worktree:no -->"])


def fake_runner(main, marker):
    work = main.parent / "astrolith-work"
    listing = f"worktree {main}\n\nworktree {work / '613-worktrees'}\n"

    def run(cmd):
        if cmd[:2] == ["gh", "api"]:
            return SimpleNamespace(returncode=0, stdout=marker, stderr="")
        if cmd[:3] == ["git", "worktree", "list"]:
            return SimpleNamespace(returncode=0, stdout=listing, stderr="")
        if "--git-common-dir" in cmd:
            return SimpleNamespace(returncode=0, stdout=str(main / ".git") + "\n", stderr="")
        return SimpleNamespace(returncode=0, stdout="", stderr="")
    return run, work / "613-worktrees"


def invoke(module, argv):
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        rc = module.main(argv)
    return rc, json.loads(buf.getvalue().strip().splitlines()[-1])


def check(name, got, want):
    if got != want:
        print(f"FAIL {name}: got {got!r}, want {want!r}")
        return False
    print(f"ok {name}")
    return True


def main():
    ok = True
    with tempfile.TemporaryDirectory() as tmp:
        main_dir = Path(tmp) / "astrolith"
        main_dir.mkdir()
        started = []
        live = set()
        launch.start = lambda cmd, cwd, log: (started.append((cmd, str(cwd))), 4242)[1]
        launch.alive = lambda pid: pid in live

        run, folder = fake_runner(main_dir, YES)
        development.run = run
        rc, out = invoke(launch, ["--issue", "613", "--agent", "claude", "--dry-run"])
        ok &= check("dry-run-command",
                    (rc, out["cwd"], out["command"][0], "#613:" in " ".join(out["command"]),
                     started), (0, str(folder), "claude", True, []))

        rc, out = invoke(launch, ["--issue", "613", "--agent", "opencode"])
        ok &= check("launch-starts-in-worktree",
                    (rc, out["pid"], started and started[0][1], started and started[0][0][:2]),
                    (0, 4242, str(folder), ["opencode", "run"]))
        live.add(4242)
        rc, out = invoke(launch, ["--list"])
        ok &= check("list-shows-running",
                    [(a["issue"], a["agent"], a["running"]) for a in out["agents"]],
                    [(613, "opencode", True)])
        rc, out = invoke(launch, ["--issue", "613", "--agent", "claude"])
        ok &= check("second-agent-refused", (rc, len(started)), (1, 1))
        live.clear()
        rc, out = invoke(launch, ["--issue", "613", "--agent", "claude"])
        ok &= check("relaunch-after-stop", (rc, len(started)), (0, 2))

        run, _ = fake_runner(main_dir, NO)
        development.run = run
        rc, out = invoke(launch, ["--issue", "613", "--agent", "claude", "--dry-run"])
        ok &= check("normal-issue-refused", rc, 1)

    ok &= check("tag-once", (notify.tagged(5, "hello"), notify.tagged(5, "#5: hi")),
                ("#5: hello", "#5: hi"))
    calls = []
    notify.run = lambda cmd: (calls.append(cmd), SimpleNamespace(returncode=0, stdout="", stderr=""))[1]
    sent = []
    notify.send_telegram = lambda token, chat, text: (sent.append(text), True)[1]
    for key in ("TELEGRAM_BOT_TOKEN", "TELEGRAM_CHAT_ID"):
        os.environ.pop(key, None)
    rc, out = invoke(notify, ["--issue", "5", "question?"])
    ok &= check("notify-issue-only-without-token",
                (rc, out["issueComment"], out["telegram"], sent), (0, True, False, []))
    os.environ["TELEGRAM_BOT_TOKEN"] = "t"
    os.environ["TELEGRAM_CHAT_ID"] = "1"
    rc, out = invoke(notify, ["--issue", "5", "question?"])
    ok &= check("notify-telegram-with-token", (rc, out["telegram"], sent), (0, True, ["#5: question?"]))
    if not ok:
        print("fixtures: FAIL")
        return 1
    print("fixtures: all ok (launcher and notify)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
