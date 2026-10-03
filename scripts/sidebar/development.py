"""Development sidebar tool: linked branches/PRs, create issue-linked branch, link PR text.

Examples:
  python scripts/sidebar/development.py list --issue 35
  python scripts/sidebar/development.py create-branch --issue 35 --slug flight-mode --base main
  python scripts/sidebar/development.py pr-body-line --issue 35 --kind parent
  python scripts/sidebar/development.py pr-body-line --issue 51 --kind task

Rule (workflow.md): NEVER use Closes/Fixes/Resolves for a PARENT issue (notion/feedback/bug).
Parents link with "Related to #N". Only task sub-issues may use "Closes #N".
JSON to stdout.
"""
import argparse
import json
import re
import subprocess
import sys


def run_gh(args):
    return subprocess.run(["gh"] + args, capture_output=True, text=True,
                          encoding="utf-8", errors="replace")


def repo():
    p = run_gh(["repo", "view", "--json", "nameWithOwner", "--jq", ".nameWithOwner"])
    return p.stdout.strip() if p.returncode == 0 else ""


def issue_labels(number):
    p = run_gh(["issue", "view", str(number), "--json", "labels", "--jq", "[.labels[].name]"])
    if p.returncode != 0:
        return []
    try:
        return json.loads(p.stdout or "[]")
    except json.JSONDecodeError:
        return []


def is_parent(number):
    return any(l in ("notion", "feedback", "bug") for l in issue_labels(number))


def cmd_list(a):
    r = repo()
    branches, prs = [], []
    # Linked branches via the supported mechanism (the issues/branches REST
    # path returns 404). One branch per line as "name<TAB>url"; keep bare
    # names. A failure surfaces instead of a silent empty list.
    b = run_gh(["issue", "develop", str(a.issue), "--list"])
    if b.returncode == 0:
        branches = [l.split()[0] for l in (b.stdout or "").splitlines()
                    if l.split()]
    else:
        print(json.dumps({"ok": False, "issue": a.issue,
                          "error": "gh issue develop --list failed: "
                                   + b.stderr.strip()[:300]}))
        return 1
    # linked PRs via timeline cross-references
    t = run_gh(["api", f"repos/{r}/issues/{a.issue}/timeline?per_page=100", "--jq",
                "[.[] | select(.event==\"cross-referenced\") | .source.issue | {number, title, state, pull_request: (.pull_request != null)}]"])
    if t.returncode == 0:
        try:
            prs = [x for x in json.loads(t.stdout or "[]") if x.get("pull_request")]
        except json.JSONDecodeError:
            prs = []
    print(json.dumps({"ok": True, "issue": a.issue, "branches": branches,
                      "linkedPRs": prs,
                      "display": f"{len(branches)} branch(es), {len(prs)} linked PR(s)"}))
    return 0


def cmd_create_branch(a):
    name = f"{a.issue}-{a.slug}"
    full = f"{a.kind}/{name}" if not a.slug.startswith(f"{a.issue}-") else f"{a.kind}/{a.slug}"
    # avoid duplicates: check local + remote (`gh branch` does not exist,
    # so the check is git-based and the compared name is the prefixed one
    # actually created)
    ex_local = subprocess.run(
        ["git", "branch", "--list", full, "--format=%(refname:short)"],
        capture_output=True, text=True, encoding="utf-8", errors="replace")
    ex_remote = subprocess.run(
        ["git", "ls-remote", "--heads", "origin", full],
        capture_output=True, text=True, encoding="utf-8", errors="replace")
    local_hit = full in (ex_local.stdout or "").splitlines()
    remote_hit = any(l.split()[-1].endswith("/" + full)
                     for l in (ex_remote.stdout or "").splitlines() if l.split())
    if local_hit or remote_hit:
        print(json.dumps({"ok": True, "action": "already_present", "branch": full}))
        return 0
    p = run_gh(["issue", "develop", str(a.issue), "--name", full, "--base", a.base,
                "--checkout"])
    if p.returncode != 0:
        # fall back to plain branch (unlinked) with explicit warning
        print(json.dumps({"ok": False, "error": p.stderr.strip()[:500],
                          "hint": f"gh issue develop failed; create manually: git checkout -b {full} {a.base}"}))
        return 1
    print(json.dumps({"ok": True, "action": "created", "branch": full,
                      "note": "linked to issue (no auto-close keyword used)"}))
    return 0


def cmd_pr_body_line(a):
    if a.kind == "parent" or is_parent(a.issue):
        line = f"Related to #{a.issue}"
        print(json.dumps({"ok": True, "issue": a.issue, "line": line,
                          "rule": "parent issues must NOT use Closes/Fixes/Resolves"}))
    else:
        line = f"Closes #{a.issue}"
        print(json.dumps({"ok": True, "issue": a.issue, "line": line,
                          "rule": "task sub-issues may use Closes"}))
    return 0


def cmd_check_body(a):
    body = a.body or ""
    body_file = getattr(a, "body_file", "") or ""
    if body_file:
        try:
            with open(body_file, encoding="utf-8", errors="replace") as f:
                body = f.read()
        except OSError as e:
            print(json.dumps({"ok": False, "error": f"cannot read body file: {e}"}))
            return 1
    m = re.findall(r"\b(?:close|closes|closed|fix|fixes|fixed|resolve|resolves|resolved)\s*:?\s*#(\d+)", body, re.I)
    bad = [int(n) for n in m if is_parent(int(n))]
    print(json.dumps({"ok": len(bad) == 0, "autoCloseRefs": [int(n) for n in m],
                      "parentRefs": bad,
                      "error": f"parent auto-close keyword targets {bad}" if bad else ""}))
    return 0 if not bad else 1


def main(argv=None):
    ap = argparse.ArgumentParser(description="Development sidebar tool")
    sub = ap.add_subparsers(dest="cmd", required=True)
    li = sub.add_parser("list"); li.add_argument("--issue", type=int, required=True)
    cb = sub.add_parser("create-branch"); cb.add_argument("--issue", type=int, required=True)
    cb.add_argument("--slug", required=True); cb.add_argument("--base", default="main")
    cb.add_argument("--kind", default="feat", choices=["feat", "fix"])
    pl = sub.add_parser("pr-body-line"); pl.add_argument("--issue", type=int, required=True)
    pl.add_argument("--kind", default="parent", choices=["parent", "task"])
    ck = sub.add_parser("check-body"); ck.add_argument("--body", default="")
    ck.add_argument("--body-file", default="")
    a = ap.parse_args(argv)
    return {"list": cmd_list, "create-branch": cmd_create_branch,
            "pr-body-line": cmd_pr_body_line, "check-body": cmd_check_body}[a.cmd](a)


if __name__ == "__main__":
    sys.exit(main())
