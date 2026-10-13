"""Fixture harness for evidence in the main checkout (issue #613, T2).

Builds a throwaway git repository with a worktree in a temporary folder
(nothing outside it is touched, E-TEST-HERMETIC). Pins:
  - from a worktree, the evidence folder is the main checkout's
    `.agent/validation/`;
  - an accepted issue's untracked evidence is deleted, its tracked
    `requirements.md` kept;
  - the bulk cleanup picks everything untracked of a closed issue and the
    older runs of an open one, never a tracked file or the newest run.

Usage:
  python scripts/validation/test_evidence.py
Exit 0 when every fixture behaves; non-zero naming the first failure.
"""
import os
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent / "sidebar"))

import evidence  # noqa: E402
import cleanup_evidence  # noqa: E402


def git(cwd, *args):
    subprocess.run(["git", "-C", str(cwd), *args], check=True, capture_output=True, text=True)


def check(name, got, want):
    if got != want:
        print(f"FAIL {name}: got {got!r}, want {want!r}")
        return False
    print(f"ok {name}")
    return True


def make_repo(tmp):
    main = Path(tmp) / "repo"
    main.mkdir()
    git(main, "init", "-q", "-b", "main")
    git(main, "config", "user.email", "t@example.com")
    git(main, "config", "user.name", "t")
    for issue in (7, 8):
        d = main / ".agent" / "validation" / f"issue-{issue}"
        d.mkdir(parents=True)
        (d / "requirements.md").write_text("req\n", encoding="utf-8")
    git(main, "add", ".")
    git(main, "commit", "-q", "-m", "init")
    for issue in (7, 8):
        d = main / ".agent" / "validation" / f"issue-{issue}"
        for run in ("20261001T000000Z", "20261002T000000Z"):
            (d / run).mkdir()
            (d / run / "report.md").write_text("r\n", encoding="utf-8")
        (d / "t2-screens").mkdir()
        (d / "t2-screens" / "a.png").write_bytes(b"x")
    wt = Path(tmp) / "work" / "7-x"
    git(main, "worktree", "add", "-q", str(wt), "-b", "feat/7-x")
    return main, wt


def main():
    ok = True
    with tempfile.TemporaryDirectory() as tmp:
        main_dir, wt = make_repo(tmp)
        same = lambda a, b: os.path.normcase(str(Path(a).resolve())) == os.path.normcase(str(Path(b).resolve()))
        ok &= check("evidence-from-worktree-is-main",
                    same(evidence.evidence_dir(wt), main_dir / ".agent" / "validation"), True)
        ok &= check("evidence-from-main-is-main",
                    same(evidence.evidence_dir(main_dir), main_dir / ".agent" / "validation"), True)

        picked = cleanup_evidence.select(main_dir, closed={8})
        names = sorted(f"{p.parent.name}/{p.name}" for p, _ in picked)
        ok &= check("cleanup-selection", names, sorted([
            "issue-7/20261001T000000Z",
            "issue-8/20261001T000000Z",
            "issue-8/20261002T000000Z",
            "issue-8/t2-screens",
        ]))

        removed = evidence.clean_issue(wt, 7)
        d7 = main_dir / ".agent" / "validation" / "issue-7"
        ok &= check("done-cleans-untracked",
                    (len(removed), sorted(p.name for p in d7.iterdir())),
                    (3, ["requirements.md"]))
    if not ok:
        print("fixtures: FAIL")
        return 1
    print("fixtures: all ok (evidence)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
