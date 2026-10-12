"""Fixture harness for the development tool's branch flow (issue #533, T1).

A fake git/gh runner replaces `development.run`, so nothing touches the
network or the real repo (E-TEST-HERMETIC). Pins:
  - create-branch never calls `gh issue develop` (a Development link
    closes the parent when its PR merges) and links by issue comment;
  - task branches start only from a parent branch, parent branches only
    from main;
  - merge-task merges --no-ff into the parent branch with no PR, closes
    the sub-issue naming the merge commit, and refuses non-task issues,
    dirty trees, and failed merges (aborting them);
  - list finds branches by name on origin.

Usage:
  python scripts/sidebar/test_development.py
Exit 0 when every fixture behaves; non-zero naming the first failure.
"""

import contextlib
import io
import json
import os
import sys
from types import SimpleNamespace

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import development  # noqa: E402


class Fake:
    """Answers commands by prefix; records every call in order."""

    def __init__(self, answers):
        self.answers = answers
        self.calls = []

    def __call__(self, cmd):
        self.calls.append(cmd)
        for prefix, (rc, out) in self.answers:
            if cmd[:len(prefix)] == prefix:
                return SimpleNamespace(returncode=rc, stdout=out, stderr="" if rc == 0 else "boom")
        return SimpleNamespace(returncode=0, stdout="", stderr="")

    def ran(self, prefix):
        return any(c[:len(prefix)] == prefix for c in self.calls)


def invoke(fake, argv):
    development.run = fake
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        rc = development.main(argv)
    return rc, json.loads(buf.getvalue().strip().splitlines()[-1])


def check(name, got, want):
    if got != want:
        print(f"FAIL {name}: got {got!r}, want {want!r}")
        return False
    print(f"ok {name}")
    return True


def main():
    ok = True

    fake = Fake([])
    rc, out = invoke(fake, ["create-branch", "--issue", "533", "--slug", "tools",
                            "--kind", "fix"])
    ok &= check("parent-branch-created", (rc, out["branch"]), (0, "fix/533-tools"))
    ok &= check("parent-branch-no-development-link",
                fake.ran(["gh", "issue", "develop"]), False)
    ok &= check("parent-branch-from-main",
                fake.ran(["git", "switch", "-c", "fix/533-tools", "main"]), True)
    ok &= check("parent-branch-pushed", fake.ran(["git", "push", "-u", "origin"]), True)
    ok &= check("parent-branch-linked-by-comment",
                fake.ran(["gh", "issue", "comment", "533"]), True)

    fake = Fake([])
    rc, out = invoke(fake, ["create-branch", "--issue", "534", "--slug", "tool",
                            "--kind", "task", "--base", "fix/533-tools"])
    ok &= check("task-branch-from-parent", (rc, out["branch"]), (0, "task/534-tool"))
    ok &= check("task-branch-switch",
                fake.ran(["git", "switch", "-c", "task/534-tool", "fix/533-tools"]), True)

    fake = Fake([])
    rc, _ = invoke(fake, ["create-branch", "--issue", "534", "--slug", "tool",
                          "--kind", "task", "--base", "main"])
    ok &= check("task-branch-refuses-main", (rc, fake.ran(["git", "switch"])), (1, False))

    fake = Fake([])
    rc, _ = invoke(fake, ["create-branch", "--issue", "533", "--slug", "x",
                          "--kind", "fix", "--base", "feat/1-y"])
    ok &= check("parent-branch-refuses-non-main", rc, 1)

    fake = Fake([(["git", "ls-remote"], (0, "abc\trefs/heads/fix/533-tools\n"))])
    rc, out = invoke(fake, ["create-branch", "--issue", "533", "--slug", "tools",
                            "--kind", "fix"])
    ok &= check("existing-branch-kept", (rc, out["action"]), (0, "already_present"))

    merge_ok = [
        (["gh", "issue", "view"], (0, '["task"]')),
        (["git", "branch", "--list"], (0, "task/534-tool\n")),
        (["git", "rev-parse", "HEAD"], (0, "f" * 40 + "\n")),
    ]
    fake = Fake(merge_ok)
    rc, out = invoke(fake, ["merge-task", "--issue", "534", "--into", "fix/533-tools"])
    ok &= check("merge-task-ok", (rc, out["merge"], out["closed"]), (0, "f" * 40, True))
    ok &= check("merge-task-no-ff",
                fake.ran(["git", "merge", "--no-ff", "-m",
                          "Merge task #534 into fix/533-tools", "task/534-tool"]), True)
    ok &= check("merge-task-no-pr", fake.ran(["gh", "pr"]), False)
    close = [c for c in fake.calls if c[:3] == ["gh", "issue", "close"]]
    ok &= check("merge-task-closes-naming-commit",
                bool(close) and ("f" * 40) in close[0][-1], True)
    ok &= check("merge-task-deletes-branch",
                fake.ran(["git", "push", "origin", "--delete", "task/534-tool"]), True)

    fake = Fake([(["gh", "issue", "view"], (0, '["bug"]'))])
    rc, _ = invoke(fake, ["merge-task", "--issue", "533", "--into", "fix/533-tools"])
    ok &= check("merge-task-refuses-parent",
                (rc, fake.ran(["gh", "issue", "close"])), (1, False))

    fake = Fake(merge_ok)
    rc, _ = invoke(fake, ["merge-task", "--issue", "534", "--into", "main"])
    ok &= check("merge-task-refuses-main", (rc, fake.ran(["git", "merge"])), (1, False))

    fake = Fake(merge_ok + [(["git", "status"], (0, " M a.py\n"))])
    rc, _ = invoke(fake, ["merge-task", "--issue", "534", "--into", "fix/533-tools"])
    ok &= check("merge-task-refuses-dirty", (rc, fake.ran(["git", "merge"])), (1, False))

    fake = Fake([(["git", "merge", "--no-ff"], (1, "CONFLICT"))] + merge_ok)
    rc, _ = invoke(fake, ["merge-task", "--issue", "534", "--into", "fix/533-tools"])
    ok &= check("merge-task-conflict-aborts",
                (rc, fake.ran(["git", "merge", "--abort"]), fake.ran(["gh", "issue", "close"])),
                (1, True, False))

    fake = Fake([(["git", "ls-remote"], (0, "a\trefs/heads/fix/533-tools\n"
                                            "b\trefs/heads/task/534-tool\n"
                                            "c\trefs/heads/feat/5330-other\n")),
                 (["gh", "issue", "develop"], (1, ""))])
    rc, out = invoke(fake, ["list", "--issue", "533"])
    ok &= check("list-by-name", (rc, out["branches"]), (0, ["fix/533-tools"]))

    if not ok:
        print("fixtures: FAIL")
        return 1
    print("fixtures: all ok (development branch flow)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
