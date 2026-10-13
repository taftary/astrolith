"""Fixture harness for verdict carry-over and the main hook (issue #613, T3).

Builds throwaway git repositories in a temporary folder (nothing outside
it is touched, E-TEST-HERMETIC). Pins:
  - a clean update from `main` that touches none of the PR's files carries
    the verdict over;
  - an update touching a file the PR changed does not;
  - a head with changes of its own beyond the update does not;
  - an old commit that is not an ancestor of the head does not;
  - the tracked pre-commit hook refuses a commit on `main` and lets a commit
    on another branch through.

Usage:
  python scripts/validation/test_carry_verdict.py
Exit 0 when every fixture behaves; non-zero naming the first failure.
"""
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[1]
sys.path.insert(0, str(HERE))

import carry_verdict  # noqa: E402


def git(cwd, *args, check=True):
    return subprocess.run(["git", "-C", str(cwd), *args], check=check,
                          capture_output=True, text=True)


def sha(cwd, ref="HEAD"):
    return git(cwd, "rev-parse", ref).stdout.strip()


def write(cwd, name, text):
    (Path(cwd) / name).write_text(text, encoding="utf-8")


def commit(cwd, msg):
    git(cwd, "add", "-A")
    git(cwd, "commit", "-q", "-m", msg)


def check(name, got, want):
    if got != want:
        print(f"FAIL {name}: got {got!r}, want {want!r}")
        return False
    print(f"ok {name}")
    return True


def repo(tmp, name):
    d = Path(tmp) / name
    d.mkdir()
    git(d, "init", "-q", "-b", "main")
    git(d, "config", "user.email", "t@example.com")
    git(d, "config", "user.name", "t")
    for f in ("a.txt", "b.txt", "c.txt"):
        write(d, f, f + "\n")
    commit(d, "init")
    return d


def carried(d, old, head):
    cwd = Path.cwd()
    import os
    os.chdir(d)
    try:
        ok, detail = carry_verdict.check_carry(old, head, "main")
    finally:
        os.chdir(cwd)
    return ok, detail


def main():
    ok = True
    with tempfile.TemporaryDirectory() as tmp:
        # Clean: the PR changes a.txt; main changes b.txt; merge main in.
        d = repo(tmp, "clean")
        git(d, "switch", "-q", "-c", "feat/1-x")
        write(d, "a.txt", "pr\n"); commit(d, "pr")
        old = sha(d)
        git(d, "switch", "-q", "main")
        write(d, "b.txt", "main\n"); commit(d, "main moves")
        git(d, "switch", "-q", "feat/1-x")
        git(d, "merge", "-q", "--no-edit", "main")
        good, detail = carried(d, old, sha(d))
        ok &= check("clean-update-carries", (good, detail.get("overlap")), (True, []))

        # Overlap: main changes the PR's file too (no conflict, other line).
        d = repo(tmp, "overlap")
        write(d, "a.txt", "one\ntwo\nthree\n"); commit(d, "lines")
        git(d, "switch", "-q", "-c", "feat/2-x")
        write(d, "a.txt", "ONE\ntwo\nthree\n"); commit(d, "pr")
        old = sha(d)
        git(d, "switch", "-q", "main")
        write(d, "a.txt", "one\ntwo\nTHREE\n"); commit(d, "main moves")
        git(d, "switch", "-q", "feat/2-x")
        git(d, "merge", "-q", "--no-edit", "main")
        good, detail = carried(d, old, sha(d))
        ok &= check("overlap-refused", (good, detail.get("overlap")), (False, ["a.txt"]))

        # Stray: the head also changes c.txt itself.
        d = repo(tmp, "stray")
        git(d, "switch", "-q", "-c", "feat/3-x")
        write(d, "a.txt", "pr\n"); commit(d, "pr")
        old = sha(d)
        git(d, "switch", "-q", "main")
        write(d, "b.txt", "main\n"); commit(d, "main moves")
        git(d, "switch", "-q", "feat/3-x")
        git(d, "merge", "-q", "--no-edit", "main")
        write(d, "c.txt", "more\n"); commit(d, "more work")
        good, detail = carried(d, old, sha(d))
        ok &= check("stray-refused", (good, detail.get("stray")), (False, ["c.txt"]))

        # Not an ancestor.
        good, detail = carried(d, sha(d, "main"), old)
        ok &= check("non-ancestor-refused", good, False)

        # The hook on main and on a branch.
        d = repo(tmp, "hook")
        hooks = d / ".githooks"
        hooks.mkdir()
        shutil.copy(REPO / ".githooks" / "pre-commit", hooks / "pre-commit")
        git(d, "config", "core.hooksPath", ".githooks")
        write(d, "a.txt", "x\n"); git(d, "add", "-A")
        on_main = git(d, "commit", "-q", "-m", "on main", check=False).returncode
        git(d, "switch", "-q", "-c", "chore/x")
        on_branch = git(d, "commit", "-q", "-m", "on branch", check=False).returncode
        ok &= check("hook-refuses-main-allows-branch", (on_main != 0, on_branch), (True, 0))
    if not ok:
        print("fixtures: FAIL")
        return 1
    print("fixtures: all ok (carry-over and hook)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
