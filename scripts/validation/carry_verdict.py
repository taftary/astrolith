"""Carries a validator verdict over a clean update from `main` (#613, P3 A).

When `main` moved after a PR was validated, the PR's agent brings the
branch up to date. The verdict for the validated commit `--from OLD` still
holds for the updated head when:
  1. OLD is an ancestor of the head (the update only added commits);
  2. every file the head changes against OLD was changed on `main` by the
     update (the PR's own changes did not move);
  3. no file the PR changed (its validated changes against its old base)
     was changed on `main` by the update (no overlap).
Then this posts `<!-- validator:carried sha=<head> from=<OLD> -->` on the
issue with both file lists. The merge gate re-checks the same rule with
git before it accepts the record, and still requires CI on the head; the
post-merge check still runs its full validation on `main`. Any overlap
means the validator runs again.

Usage:
  python scripts/validation/carry_verdict.py --issue N --pr P --from OLD [--main origin/main]
"""
import argparse
import json
import re
import subprocess
import sys

CARRIED = re.compile(r"<!--\s*validator:carried\s+sha=([0-9a-f]{40})\s+from=([0-9a-f]{40})\s*-->")


def run(cmd):
    """Run one git or gh command; tests replace this with a fake runner."""
    return subprocess.run(cmd, capture_output=True, text=True,
                          encoding="utf-8", errors="replace")


def out(cmd):
    p = run(cmd)
    return (p.stdout or "").strip() if p.returncode == 0 else None


def files(a, b):
    """Files changed between two commits (empty set when equal)."""
    text = out(["git", "diff", "--name-only", a, b])
    return None if text is None else {f for f in text.splitlines() if f}


def check_carry(old, head, main_ref="origin/main"):
    """(ok, detail) of the carry-over rule for OLD -> head."""
    if run(["git", "merge-base", "--is-ancestor", old, head]).returncode != 0:
        return False, {"reason": f"{old[:7]} is not an ancestor of {head[:7]}"}
    old_base = out(["git", "merge-base", old, main_ref])
    new_base = out(["git", "merge-base", head, main_ref])
    if not old_base or not new_base:
        return False, {"reason": "cannot find the merge bases with main"}
    pr_files = files(old_base, old)
    main_files = files(old_base, new_base)
    moved = files(old, head)
    if pr_files is None or main_files is None or moved is None:
        return False, {"reason": "git diff failed"}
    overlap = sorted(pr_files & main_files)
    stray = sorted(moved - main_files)
    detail = {"prFiles": sorted(pr_files), "mainFiles": sorted(main_files),
              "overlap": overlap, "stray": stray}
    if overlap:
        return False, {**detail, "reason": "the update touched files the PR changed"}
    if stray:
        return False, {**detail, "reason": "the head changes files beyond the update from main"}
    return True, detail


def record(head, old, detail):
    """The issue comment recording a carried verdict."""
    def listing(name, items):
        return f"- {name}: " + (", ".join(f"`{f}`" for f in items) if items else "none")
    return "\n".join([
        f"<!-- validator:carried sha={head} from={old} -->",
        f"Validator verdict carried over from {old} to {head} (clean update from `main`, no overlap; #613 P3).",
        listing("PR files (validated)", detail["prFiles"]),
        listing("Files the update brought from main", detail["mainFiles"]),
        "- Overlap: none",
    ])


def main(argv=None):
    ap = argparse.ArgumentParser(description="Carry a verdict over a clean update from main")
    ap.add_argument("--issue", type=int, required=True)
    ap.add_argument("--pr", type=int, required=True)
    ap.add_argument("--from", dest="old", required=True)
    ap.add_argument("--main", default="origin/main")
    a = ap.parse_args(argv)
    head = out(["gh", "pr", "view", str(a.pr), "--json", "headRefOid", "--jq", ".headRefOid"])
    if not head or not re.fullmatch(r"[0-9a-f]{40}", head):
        print(json.dumps({"ok": False, "error": f"cannot read PR #{a.pr} head"}))
        return 1
    run(["git", "fetch", "origin"])
    ok, detail = check_carry(a.old, head, a.main)
    if not ok:
        print(json.dumps({"ok": False, "head": head, "from": a.old, **detail,
                          "next": "run the validator again for the head"}))
        return 1
    p = run(["gh", "issue", "comment", str(a.issue), "--body", record(head, a.old, detail)])
    if p.returncode != 0:
        print(json.dumps({"ok": False, "error": "gh issue comment failed"}))
        return 1
    print(json.dumps({"ok": True, "head": head, "from": a.old, **detail}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
