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

Worktrees (#613): the owner chooses per piece of work, recorded on the issue
by `choose` as a marker comment. With "worktree", `create-branch` makes the
parent branch in its own `../astrolith-work/<N>-<slug>` worktree (free-space
floor, long paths, its own game settings, shared agent memory); task
branches are made inside that worktree, and `create-branch` and
`merge-task` refuse to act anywhere else. `check` switches the issue's
`-check` worktree to a commit for the validator; `remove` removes both after
the merge. With "normal" (or no choice recorded) everything works in the
main checkout as before, one piece of normal work at a time.
JSON to stdout.
"""
import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import worktrees  # noqa: E402

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


def comment_bodies(number):
    """Every comment body on `number`, oldest first ([] on failure)."""
    p = run_gh(["api", f"repos/{repo()}/issues/{number}/comments", "--paginate",
                "--jq", "[.[].body]"])
    if p.returncode != 0:
        return []
    bodies = []
    for chunk in (p.stdout or "").splitlines():
        try:
            bodies.extend(json.loads(chunk))
        except json.JSONDecodeError:
            continue
    return bodies


def way_of(number):
    """The owner's recorded choice for `number`: "worktree" or "normal"."""
    return "worktree" if worktrees.marker(comment_bodies(number)) == "yes" else "normal"


def git_out(args):
    p = run_git(args)
    return (p.stdout or "").strip() if p.returncode == 0 else ""


def main_checkout():
    """The main checkout, whichever worktree this runs in."""
    common = git_out(["rev-parse", "--path-format=absolute", "--git-common-dir"])
    return worktrees.main_checkout(common) if common else Path.cwd()


def here():
    """The top folder of the checkout this runs in."""
    top = git_out(["rev-parse", "--show-toplevel"])
    return Path(top) if top else Path.cwd()


def same_folder(a, b):
    return os.path.normcase(os.path.abspath(str(a))) == os.path.normcase(os.path.abspath(str(b)))


def parent_issue(branch):
    """`feat/613-x` -> 613; None for anything else."""
    m = re.match(r"^(?:feat|fix)/([0-9]+)-", branch or "")
    return int(m.group(1)) if m else None


def parent_folder(main, branch):
    """The worktree a parent branch lives in: `../astrolith-work/<N>-<slug>`."""
    return worktrees.work_root(main) / branch.split("/", 1)[1]


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
    owner = a.issue if a.kind != "task" else parent_issue(a.base)
    way = way_of(owner) if owner else "normal"
    main = main_checkout()
    folder = None
    if way == "worktree" and a.kind != "task":
        folder = worktrees.issue_folder(main, a.issue, a.slug)
        free = worktrees.free_bytes(worktrees.work_root(main))
        if free < worktrees.FLOOR_BYTES:
            return fail(issue=a.issue, freeGB=round(free / 1024 ** 3, 1),
                        error=f"only {free / 1024 ** 3:.1f} GB free; a worktree needs "
                              f"{worktrees.FLOOR_BYTES // 1024 ** 3} GB (free space first)")
        run_git(["config", "core.longpaths", "true"])
        p = run_git(["worktree", "add", str(folder), "-b", full, a.base])
        if p.returncode != 0:
            return fail(error=f"git worktree add failed: {p.stderr.strip()[:300]}")
        worktrees.write_cargo_env(folder)
        worktrees.link_memory(main, folder)
        p = run_git(["-C", str(folder), "push", "-u", "origin", full])
    elif way == "worktree":
        # A task branch is made inside its parent's worktree.
        expected = parent_folder(main, a.base)
        if not same_folder(here(), expected):
            return fail(issue=a.issue, worktree=str(expected),
                        error=f"run this inside the parent's worktree {expected}")
        p = run_git(["switch", "-c", full, a.base])
        if p.returncode != 0:
            return fail(error=f"git switch -c {full} {a.base} failed: {p.stderr.strip()[:300]}")
        p = run_git(["push", "-u", "origin", full])
    else:
        if a.kind != "task":
            busy = git_out(["-C", str(main), "branch", "--show-current"])
            if busy and busy != "main":
                return fail(issue=a.issue, busy=busy,
                            error=f"the main checkout is busy on {busy}; normal work runs "
                                  "one at a time, so choose a worktree for this one")
        p = run_git(["switch", "-c", full, a.base])
        if p.returncode != 0:
            return fail(error=f"git switch -c {full} {a.base} failed: {p.stderr.strip()[:300]}")
        p = run_git(["push", "-u", "origin", full])
    if p.returncode != 0:
        return fail(branch=full, error=f"git push failed: {p.stderr.strip()[:300]}")
    # The link is a comment, not the Development sidebar (see module doc).
    where = f" in worktree `{folder}`" if folder else ""
    run_gh(["issue", "comment", str(a.issue), "--body",
            f"Branch: `{full}` (from `{a.base}`){where}."])
    out = {"ok": True, "action": "created", "branch": full, "base": a.base, "way": way,
           "note": "plain git branch, linked by comment (no Development link)"}
    if folder:
        out["worktree"] = str(folder)
    print(json.dumps(out))
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
    owner = parent_issue(a.into)
    if owner and way_of(owner) == "worktree":
        expected = parent_folder(main_checkout(), a.into)
        if not same_folder(here(), expected):
            return fail(issue=a.issue, worktree=str(expected),
                        error=f"run this inside the parent's worktree {expected}")
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


def cmd_choose(a):
    """Records the owner's choice for a piece of work (Spec v4 item 0)."""
    p = run_gh(["issue", "comment", str(a.issue), "--body", worktrees.marker_comment(a.mode)])
    if p.returncode != 0:
        return fail(issue=a.issue, error=f"gh issue comment failed: {p.stderr.strip()[:300]}")
    print(json.dumps({"ok": True, "issue": a.issue, "way": a.mode}))
    return 0


def issue_worktree(main, issue):
    """The issue's worktree folder from `git worktree list` (not `-check`)."""
    listing = git_out(["worktree", "list", "--porcelain"])
    root = worktrees.work_root(main)
    for line in listing.splitlines():
        if not line.startswith("worktree "):
            continue
        path = Path(line[len("worktree "):].strip())
        if (same_folder(path.parent, root) and path.name.startswith(f"{issue}-")
                and not path.name.endswith("-check")):
            return path
    return None


def cmd_check(a):
    """Switches the issue's validator worktree to `sha`, detached (item 3)."""
    main = main_checkout()
    folder = issue_worktree(main, a.issue)
    if folder is None:
        return fail(issue=a.issue, error="no worktree for this issue; normal work "
                                         "validates in the main checkout")
    check = worktrees.check_folder(folder)
    run_git(["fetch", "origin"])
    if any(same_folder(check, line[len("worktree "):].strip())
           for line in git_out(["worktree", "list", "--porcelain"]).splitlines()
           if line.startswith("worktree ")):
        p = run_git(["-C", str(check), "checkout", "--detach", a.sha])
    else:
        p = run_git(["worktree", "add", "--detach", str(check), a.sha])
        if p.returncode == 0:
            worktrees.write_cargo_env(check)
    if p.returncode != 0:
        return fail(issue=a.issue, error=f"cannot put the check worktree at {a.sha}: "
                                         f"{p.stderr.strip()[:300]}")
    print(json.dumps({"ok": True, "issue": a.issue, "check": str(check), "sha": a.sha}))
    return 0


def cmd_remove(a):
    """Removes an issue's worktrees and their builds after the merge (item 4)."""
    main = main_checkout()
    folder = issue_worktree(main, a.issue)
    if folder is None:
        print(json.dumps({"ok": True, "issue": a.issue, "action": "nothing_to_remove"}))
        return 0
    removed = []
    for path in (worktrees.check_folder(folder), folder):
        p = run_git(["worktree", "remove", "--force", str(path)])
        if p.returncode == 0:
            removed.append(str(path))
            worktrees.unlink_memory(path)
    run_git(["worktree", "prune"])
    print(json.dumps({"ok": True, "issue": a.issue, "action": "removed", "removed": removed}))
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
    ch = sub.add_parser("choose"); ch.add_argument("--issue", type=int, required=True)
    ch.add_argument("--mode", required=True, choices=["worktree", "normal"])
    cv = sub.add_parser("check"); cv.add_argument("--issue", type=int, required=True)
    cv.add_argument("--sha", required=True)
    rm = sub.add_parser("remove"); rm.add_argument("--issue", type=int, required=True)
    a = ap.parse_args(argv)
    return {"list": cmd_list, "create-branch": cmd_create_branch, "merge-task": cmd_merge_task,
            "pr-body-line": cmd_pr_body_line, "check-body": cmd_check_body,
            "choose": cmd_choose, "check": cmd_check, "remove": cmd_remove}[a.cmd](a)


if __name__ == "__main__":
    sys.exit(main())
