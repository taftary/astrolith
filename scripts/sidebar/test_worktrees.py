"""Fixture harness for worktrees per piece of work (issue #613, T1).

A fake git/gh runner replaces `development.run` and the file-system steps
of `worktrees` are replaced by recorders, so nothing touches the network,
the real repo, or the disk outside a temporary folder (E-TEST-HERMETIC).
Pins:
  - the marker records the owner's choice and the newest one wins;
  - with "worktree", create-branch makes the parent branch in
    `../astrolith-work/<N>-<slug>` (long paths, cargo env, memory link) and
    refuses below the free-space floor;
  - task branches and merge-task refuse to act outside the parent's worktree;
  - with "normal", a second piece of normal work is refused while the main
    checkout is busy on another branch;
  - check puts the `-check` worktree at the commit, detached; remove takes
    both worktrees away;
  - the cargo config points ASTROLITH_SETTINGS inside the worktree.

Usage:
  python scripts/sidebar/test_worktrees.py
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
import worktrees  # noqa: E402

MAIN = Path(tempfile.gettempdir()) / "fixture-repos" / "astrolith"
WORK = MAIN.parent / "astrolith-work"
YES = json.dumps(["<!-- worktree:yes -->\nWay of working: **new worktree**."])
NO = json.dumps(["<!-- worktree:no -->"])


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


class Recorder:
    """Stands in for the file-system steps and records them."""

    def __init__(self, free=100 * 1024 ** 3):
        self.free = free
        self.cargo = []
        self.linked = []
        self.unlinked = []

    def install(self):
        worktrees.free_bytes = lambda path: self.free
        worktrees.write_cargo_env = lambda folder: self.cargo.append(str(folder))
        worktrees.link_memory = lambda main, folder: self.linked.append(str(folder))
        worktrees.unlink_memory = lambda folder: self.unlinked.append(str(folder))


def base(answers, here=MAIN, current="main"):
    """Common answers: the main checkout, where this runs, its branch."""
    return answers + [
        (["git", "rev-parse", "--path-format=absolute", "--git-common-dir"],
         (0, str(MAIN / ".git") + "\n")),
        (["git", "rev-parse", "--show-toplevel"], (0, str(here) + "\n")),
        (["git", "-C", str(MAIN), "branch", "--show-current"], (0, current + "\n")),
    ]


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
    ok &= check("marker-none", worktrees.marker(["hello"]), None)
    ok &= check("marker-newest-wins",
                worktrees.marker(["<!-- worktree:yes -->", "x", "<!-- worktree:no -->"]), "no")
    ok &= check("marker-comment-round-trip",
                worktrees.marker([worktrees.marker_comment("worktree")]), "yes")
    ok &= check("folders",
                (worktrees.issue_folder(MAIN, 613, "worktrees"),
                 worktrees.check_folder(WORK / "613-worktrees"),
                 worktrees.branch_folder(MAIN, "chore/tidy-docs")),
                (WORK / "613-worktrees", WORK / "613-worktrees-check", WORK / "chore-tidy-docs"))
    ok &= check("main-from-common-dir", worktrees.main_checkout(str(MAIN / ".git")), MAIN)

    # choose records the marker.
    fake = Fake([])
    rc, out = invoke(fake, ["choose", "--issue", "613", "--mode", "worktree"])
    posted = [c for c in fake.calls if c[:3] == ["gh", "issue", "comment"]]
    ok &= check("choose-posts-marker",
                (rc, out["way"], bool(posted) and "worktree:yes" in posted[0][-1]),
                (0, "worktree", True))

    # Worktree: the parent branch gets its own folder.
    rec = Recorder(); rec.install()
    fake = Fake(base([(["gh", "api"], (0, YES))]))
    rc, out = invoke(fake, ["create-branch", "--issue", "613", "--slug", "worktrees"])
    folder = str(WORK / "613-worktrees")
    ok &= check("worktree-created", (rc, out["way"], out.get("worktree")), (0, "worktree", folder))
    ok &= check("worktree-add",
                fake.ran(["git", "worktree", "add", folder, "-b", "feat/613-worktrees", "main"]), True)
    ok &= check("worktree-long-paths", fake.ran(["git", "config", "core.longpaths", "true"]), True)
    ok &= check("worktree-no-switch-in-main", fake.ran(["git", "switch"]), False)
    ok &= check("worktree-cargo-env-and-memory", (rec.cargo, rec.linked), ([folder], [folder]))
    ok &= check("worktree-pushed", fake.ran(["git", "-C", folder, "push", "-u", "origin"]), True)

    # Worktree: refused below the free-space floor, nothing created.
    rec = Recorder(free=3 * 1024 ** 3); rec.install()
    fake = Fake(base([(["gh", "api"], (0, YES))]))
    rc, out = invoke(fake, ["create-branch", "--issue", "613", "--slug", "worktrees"])
    ok &= check("worktree-disk-floor",
                (rc, out.get("freeGB"), fake.ran(["git", "worktree", "add"])), (1, 3.0, False))

    # Worktree: a task branch only inside the parent's worktree.
    rec = Recorder(); rec.install()
    fake = Fake(base([(["gh", "api"], (0, YES))], here=MAIN))
    rc, out = invoke(fake, ["create-branch", "--issue", "614", "--slug", "helper",
                            "--kind", "task", "--base", "feat/613-worktrees"])
    ok &= check("task-refused-outside-worktree",
                (rc, out.get("worktree"), fake.ran(["git", "switch"])), (1, folder, False))
    fake = Fake(base([(["gh", "api"], (0, YES))], here=WORK / "613-worktrees"))
    rc, out = invoke(fake, ["create-branch", "--issue", "614", "--slug", "helper",
                            "--kind", "task", "--base", "feat/613-worktrees"])
    ok &= check("task-inside-worktree",
                (rc, fake.ran(["git", "switch", "-c", "task/614-helper", "feat/613-worktrees"])),
                (0, True))

    # Worktree: merge-task only inside the parent's worktree.
    merge_ok = [
        (["gh", "issue", "view"], (0, '["task"]')),
        (["git", "branch", "--list"], (0, "task/614-helper\n")),
        (["gh", "api"], (0, YES)),
    ]
    fake = Fake(base(merge_ok, here=MAIN))
    rc, _ = invoke(fake, ["merge-task", "--issue", "614", "--into", "feat/613-worktrees"])
    ok &= check("merge-task-refused-outside-worktree", (rc, fake.ran(["git", "merge"])), (1, False))
    fake = Fake(base(merge_ok, here=WORK / "613-worktrees"))
    rc, _ = invoke(fake, ["merge-task", "--issue", "614", "--into", "feat/613-worktrees"])
    ok &= check("merge-task-inside-worktree", (rc, fake.ran(["git", "merge", "--no-ff"])), (0, True))

    # Normal: one piece of normal work at a time.
    fake = Fake(base([(["gh", "api"], (0, NO))], current="feat/607-player-target"))
    rc, out = invoke(fake, ["create-branch", "--issue", "620", "--slug", "x"])
    ok &= check("normal-refused-when-busy",
                (rc, out.get("busy"), fake.ran(["git", "switch"])),
                (1, "feat/607-player-target", False))
    fake = Fake(base([(["gh", "api"], (0, NO))], current="main"))
    rc, out = invoke(fake, ["create-branch", "--issue", "620", "--slug", "x"])
    ok &= check("normal-as-today",
                (rc, out["way"], fake.ran(["git", "switch", "-c", "feat/620-x", "main"]),
                 fake.ran(["git", "worktree"])), (0, "normal", True, False))

    # check: the validator worktree at the commit, detached.
    listing = f"worktree {MAIN}\nbranch refs/heads/main\n\nworktree {WORK / '613-worktrees'}\n"
    rec = Recorder(); rec.install()
    fake = Fake(base([(["git", "worktree", "list"], (0, listing))]))
    rc, out = invoke(fake, ["check", "--issue", "613", "--sha", "a" * 40])
    check_dir = str(WORK / "613-worktrees-check")
    ok &= check("check-created-detached",
                (rc, out.get("check"),
                 fake.ran(["git", "worktree", "add", "--detach", check_dir, "a" * 40])),
                (0, check_dir, True))
    listing2 = listing + f"\nworktree {check_dir}\ndetached\n"
    fake = Fake(base([(["git", "worktree", "list"], (0, listing2))]))
    rc, _ = invoke(fake, ["check", "--issue", "613", "--sha", "b" * 40])
    ok &= check("check-reused",
                (rc, fake.ran(["git", "-C", check_dir, "checkout", "--detach", "b" * 40]),
                 fake.ran(["git", "worktree", "add"])), (0, True, False))
    fake = Fake(base([(["git", "worktree", "list"], (0, f"worktree {MAIN}\n"))]))
    rc, _ = invoke(fake, ["check", "--issue", "620", "--sha", "c" * 40])
    ok &= check("check-normal-refused", rc, 1)

    # remove: both worktrees and their memory links.
    rec = Recorder(); rec.install()
    fake = Fake(base([(["git", "worktree", "list"], (0, listing2))]))
    rc, out = invoke(fake, ["remove", "--issue", "613"])
    ok &= check("remove-both",
                (rc, sorted(out["removed"]), sorted(rec.unlinked)),
                (0, sorted([check_dir, folder]), sorted([check_dir, folder])))

    # The real cargo config, in a temporary folder.
    importlib_reload()
    with tempfile.TemporaryDirectory() as tmp:
        config = worktrees.write_cargo_env(tmp)
        text = Path(config).read_text(encoding="utf-8")
        ok &= check("cargo-env-settings",
                    ("ASTROLITH_SETTINGS" in text and worktrees.SETTINGS_FILE in text
                     and "relative = true" in text), True)
    ok &= check("project-key",
                worktrees.project_key(r"C:\Users\x\source\repos\astrolith-work\613-a"),
                "C--Users-x-source-repos-astrolith-work-613-a")

    if not ok:
        print("fixtures: FAIL")
        return 1
    print("fixtures: all ok (worktrees)")
    return 0


def importlib_reload():
    """Brings back the real file-system steps after the recorders."""
    import importlib
    importlib.reload(worktrees)


if __name__ == "__main__":
    sys.exit(main())
