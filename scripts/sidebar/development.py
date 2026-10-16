"""Development sidebar tool: branches, sub-issue merges, PR link text.

Examples:
  python scripts/sidebar/development.py list --issue 35
  python scripts/sidebar/development.py create-branch --issue 35 --slug flight-mode --base main
  python scripts/sidebar/development.py create-branch --issue 36 --slug parser --kind task --base feat/35-flight-mode
  python scripts/sidebar/development.py merge-task --issue 36 --into feat/35-flight-mode
  python scripts/sidebar/development.py pr-body-line --issue 35 --kind parent
  python scripts/sidebar/development.py pr-body-line --issue 51 --kind task

Rule (workflow.md): NEVER use Closes/Fixes/Resolves for a PARENT issue (notion/feedback/bug).
Parents link with "Related to #N". Only task sub-issues may use "Closes #N".

Branches are made with plain git and linked by an issue comment, never through
`gh issue develop`: GitHub closes a Development-linked issue when a PR from
that branch merges, and parent issues must never close automatically (#533).
Each sub-issue works on `task/<sub>-<slug>` made from the parent branch and is
merged back with `merge-task` (no PR); the merge closes the sub-issue.
JSON to stdout.
"""
import argparse
import json
import re
import subprocess
import sys

KINDS = ("feat", "fix", "task")
PARENT_BRANCH = re.compile(r"^(feat|fix)/[0-9]+-[a-z0-9-]+$")


def run(cmd):
    """Run one git or gh command; tests replace this with a fake runner."""
    return subprocess.run(cmd, capture_output=True, text=True,
                          encoding="utf-8", errors="replace")


def run_gh(args):
    return run(["gh"] + args)


def run_git(args):
    return run(["git"] + args)


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


def branch_name(kind, issue, slug):
    """`<kind>/<issue>-<slug>`, accepting a slug that already starts with the number."""
    name = slug if slug.startswith(f"{issue}-") else f"{issue}-{slug}"
    return f"{kind}/{name}"


def remote_branches(issue):
    """Branch names on origin made for `issue` (`<kind>/<issue>-...`), or None on failure."""
    p = run_git(["ls-remote", "--heads", "origin"])
    if p.returncode != 0:
        return None
    out = []
    for line in (p.stdout or "").splitlines():
        parts = line.split()
        if not parts:
            continue
        name = parts[-1].removeprefix("refs/heads/")
        kind, _, rest = name.partition("/")
        if kind in KINDS and rest.startswith(f"{issue}-"):
            out.append(name)
    return out


def local_branches(pattern):
    p = run_git(["branch", "--list", pattern, "--format=%(refname:short)"])
    return [l.strip() for l in (p.stdout or "").splitlines() if l.strip()]


def fail(**fields):
    print(json.dumps({"ok": False, **fields}))
    return 1


def cmd_list(a):
    r = repo()
    named = remote_branches(a.issue)
    if named is None:
        return fail(issue=a.issue, error="git ls-remote failed")
    # Older branches were linked through the Development sidebar; keep
    # reading those links, but a failure there no longer hides the
    # name-based list.
    linked = []
    b = run_gh(["issue", "develop", str(a.issue), "--list"])
    if b.returncode == 0:
        linked = [l.split()[0] for l in (b.stdout or "").splitlines() if l.split()]
    branches = list(dict.fromkeys(named + linked))
    prs = []
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
    full = branch_name(a.kind, a.issue, a.slug)
    if a.kind == "task" and not PARENT_BRANCH.match(a.base):
        return fail(error=f"a task branch starts from its parent branch "
                          f"(feat/<N>-... or fix/<N>-...), not {a.base!r}")
    if a.kind != "task" and a.base != "main":
        return fail(error=f"a {a.kind} branch starts from main, not {a.base!r}")
    remote = remote_branches(a.issue) or []
    if full in local_branches(full) or full in remote:
        print(json.dumps({"ok": True, "action": "already_present", "branch": full}))
        return 0
    p = run_git(["switch", "-c", full, a.base])
    if p.returncode != 0:
        return fail(error=f"git switch -c {full} {a.base} failed: {p.stderr.strip()[:300]}")
    p = run_git(["push", "-u", "origin", full])
    if p.returncode != 0:
        return fail(branch=full, error=f"git push failed: {p.stderr.strip()[:300]}")
    # The link is a comment, not the Development sidebar (see module doc).
    run_gh(["issue", "comment", str(a.issue), "--body",
            f"Branch: `{full}` (from `{a.base}`)."])
    print(json.dumps({"ok": True, "action": "created", "branch": full, "base": a.base,
                      "note": "plain git branch, linked by comment (no Development link)"}))
    return 0


def cmd_merge_task(a):
    if not PARENT_BRANCH.match(a.into):
        return fail(error=f"merge-task merges into a parent branch "
                          f"(feat/<N>-... or fix/<N>-...), not {a.into!r}")
    if "task" not in issue_labels(a.issue):
        return fail(issue=a.issue, error="merge-task only closes `task` sub-issues")
    found = local_branches(f"task/{a.issue}-*")
    if len(found) != 1:
        return fail(issue=a.issue, error=f"expected one local task/{a.issue}-* branch, found {found}")
    task = found[0]
    dirty = run_git(["status", "--porcelain", "--untracked-files=no"])
    if dirty.returncode != 0 or (dirty.stdout or "").strip():
        return fail(issue=a.issue, error="tracked tree is not clean; commit first")
    p = run_git(["switch", a.into])
    if p.returncode != 0:
        return fail(error=f"git switch {a.into} failed: {p.stderr.strip()[:300]}")
    p = run_git(["merge", "--no-ff", "-m", f"Merge task #{a.issue} into {a.into}", task])
    if p.returncode != 0:
        run_git(["merge", "--abort"])
        return fail(issue=a.issue, error=f"merge of {task} failed and was aborted: "
                                         f"{((p.stdout or '') + (p.stderr or '')).strip()[:300]}")
    sha = (run_git(["rev-parse", "HEAD"]).stdout or "").strip()
    p = run_git(["push", "origin", a.into])
    if p.returncode != 0:
        return fail(issue=a.issue, merge=sha, error=f"git push failed: {p.stderr.strip()[:300]}")
    p = run_gh(["issue", "close", str(a.issue), "--comment",
                f"Done: `{task}` merged into `{a.into}` at {sha} (no PR)."])
    if p.returncode != 0:
        return fail(issue=a.issue, merge=sha, error=f"gh issue close failed: {p.stderr.strip()[:300]}")
    run_git(["branch", "-d", task])
    run_git(["push", "origin", "--delete", task])
    print(json.dumps({"ok": True, "action": "merged", "issue": a.issue, "branch": task,
                      "into": a.into, "merge": sha, "closed": True}))
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
    cb.add_argument("--kind", default="feat", choices=list(KINDS))
    mt = sub.add_parser("merge-task"); mt.add_argument("--issue", type=int, required=True)
    mt.add_argument("--into", required=True)
    pl = sub.add_parser("pr-body-line"); pl.add_argument("--issue", type=int, required=True)
    pl.add_argument("--kind", default="parent", choices=["parent", "task"])
    ck = sub.add_parser("check-body"); ck.add_argument("--body", default="")
    ck.add_argument("--body-file", default="")
    a = ap.parse_args(argv)
    return {"list": cmd_list, "create-branch": cmd_create_branch, "merge-task": cmd_merge_task,
            "pr-body-line": cmd_pr_body_line, "check-body": cmd_check_body}[a.cmd](a)


if __name__ == "__main__":
    sys.exit(main())
