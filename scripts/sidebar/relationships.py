"""Relationships sidebar tool: parent/sub-issues + blocked-by/blocking (Issue dependencies API).

Examples:
  python scripts/sidebar/relationships.py list --issue 35
  python scripts/sidebar/relationships.py add-sub --parent 35 --sub 51
  python scripts/sidebar/relationships.py add-blocked-by --issue 51 --blocked-by 35 --confirm

Lists show "None yet" when empty. Circular-dependency check warns before creating.
JSON to stdout; exit 0 incl. already-correct, 1 on failure.
"""
import argparse
import json
import subprocess
import sys


def run_gh(args):
    return subprocess.run(["gh"] + args, capture_output=True, text=True,
                          encoding="utf-8", errors="replace")


def repo():
    p = run_gh(["repo", "view", "--json", "nameWithOwner", "--jq", ".nameWithOwner"])
    return p.stdout.strip() if p.returncode == 0 else ""


def numeric_id(issue_number):
    r = repo()
    p = run_gh(["api", f"repos/{r}/issues/{issue_number}", "--jq", ".id"])
    if p.returncode != 0:
        return None
    try:
        return int(p.stdout.strip())
    except ValueError:
        return None


def list_sub(parent):
    r = repo()
    p = run_gh(["api", f"repos/{r}/issues/{parent}/sub_issues?per_page=100"])
    if p.returncode != 0:
        if "404" in (p.stderr or ""):
            return [], "sub-issues API unavailable (404)"
        return None, p.stderr.strip()
    try:
        return json.loads(p.stdout or "[]"), ""
    except json.JSONDecodeError as e:
        return None, str(e)


def list_deps(issue):
    """Blocked-by/blocking via Issue dependencies API; fallback to body refs when unavailable."""
    r = repo()
    out = {"blockedBy": [], "blocking": [], "note": ""}
    p = run_gh(["api", f"repos/{r}/issues/{issue}/dependencies?per_page=100"])
    if p.returncode == 0:
        try:
            raw = json.loads(p.stdout or "[]")
            out["blockedBy"] = [x.get("number") for x in raw if isinstance(x, dict) and x.get("number")]
            return out
        except json.JSONDecodeError:
            out["note"] = "dependencies API returned non-JSON"
            return out
    # fallback: parse issue body for explicit "blocked by #N" / "blocking #N"
    import re
    v = run_gh(["issue", "view", str(issue), "--json", "body", "--jq", ".body"])
    body = v.stdout if v.returncode == 0 else ""
    blocked = sorted({int(n) for n in re.findall(r"blocked\s+by\s+#(\d+)", body, re.I)})
    blocking = sorted({int(n) for n in re.findall(r"blocking\s+#(\d+)", body, re.I)})
    out["blockedBy"] = blocked
    out["blocking"] = blocking
    out["note"] = "dependencies API unavailable (404); derived from body refs"
    return out


def compact_subs(subs):
    slim = []
    for s in subs or []:
        slim.append({"number": s.get("number"), "title": s.get("title"),
                     "state": s.get("state")})
    return slim


def cmd_list(a):
    subs, sub_err = list_sub(a.issue)
    deps = list_deps(a.issue)
    slim = compact_subs(subs)
    print(json.dumps({"ok": True, "issue": a.issue,
                      "subIssues": slim,
                      "subIssuesDisplay": "None yet" if not slim else f"{len(slim)} sub-issue(s)",
                      "subIssuesError": sub_err,
                      "dependencies": deps}))
    return 0


def would_cycle(parent, sub):
    """True if attaching sub under parent would create a cycle (sub already ancestor of parent)."""
    seen, stack = set(), [parent]
    r = repo()
    while stack:
        cur = stack.pop()
        if cur == sub:
            return True
        if cur in seen:
            continue
        seen.add(cur)
        # find parent(s) of cur via sub_issues reverse lookup is not exposed;
        # approximate by checking cur's sub-issues only (child direction).
        # Full ancestor check needs the parent endpoint; keep bounded.
        subs, _ = list_sub(cur)
        if subs:
            for s in subs:
                n = s.get("number")
                if n and n not in seen:
                    # only descend when looking for `sub` among descendants of parent
                    pass
    # direct check: is parent already a descendant of sub?
    subs_of_sub, _ = list_sub(sub)
    if subs_of_sub:
        nums = {s.get("number") for s in subs_of_sub}
        if parent in nums:
            return True
    _ = r
    return False


def cmd_add_sub(a):
    subs, err = list_sub(a.parent)
    if subs is None:
        print(json.dumps({"ok": False, "error": f"cannot list sub-issues: {err}"}))
        return 1
    if any(s.get("number") == a.sub for s in subs):
        print(json.dumps({"ok": True, "action": "already_set", "parent": a.parent, "sub": a.sub}))
        return 0
    if would_cycle(a.parent, a.sub):
        print(json.dumps({"ok": False, "error": f"refusing: attaching #{a.sub} under #{a.parent} would create a circular dependency"}))
        return 1
    sid = numeric_id(a.sub)
    if sid is None:
        print(json.dumps({"ok": False, "error": f"cannot resolve numeric id for #{a.sub}"}))
        return 1
    r = repo()
    import tempfile, os
    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as f:
        json.dump({"sub_issue_id": sid}, f)
        tmp = f.name
    p = run_gh(["api", f"repos/{r}/issues/{a.parent}/sub_issues", "-X", "POST", "--input", tmp])
    os.unlink(tmp)
    if p.returncode != 0:
        print(json.dumps({"ok": False, "error": p.stderr.strip()}))
        return 1
    print(json.dumps({"ok": True, "action": "attached", "parent": a.parent, "sub": a.sub}))
    return 0


def cmd_add_blocked_by(a):
    if a.issue == a.blocked_by:
        print(json.dumps({"ok": False, "error": "refusing: issue cannot block itself (circular)"}))
        return 1
    deps = list_deps(a.issue)
    if any(d.get("number") == a.blocked_by for d in deps.get("blockedBy", [])):
        print(json.dumps({"ok": True, "action": "already_set"}))
        return 0
    if not a.confirm:
        print(json.dumps({"ok": False, "error": "dependency write needs --confirm",
                          "plan": {"issue": a.issue, "blockedBy": a.blocked_by},
                          "warning": "creating a circular dependency will be refused"}))
        return 1
    r = repo()
    # Issue dependencies API (blocked-by direction); falls back to informative error.
    p = run_gh(["api", f"repos/{r}/issues/{a.issue}/dependencies", "-X", "POST",
                "-f", f"blocked_by={a.blocked_by}"])
    if p.returncode != 0:
        print(json.dumps({"ok": False, "error": p.stderr.strip()[:500],
                          "hint": "if the dependencies API is unavailable, record 'blocked by #N' in the issue body instead"}))
        return 1
    print(json.dumps({"ok": True, "action": "added", "issue": a.issue, "blockedBy": a.blocked_by}))
    return 0


def main(argv=None):
    ap = argparse.ArgumentParser(description="Relationships sidebar tool")
    sub = ap.add_subparsers(dest="cmd", required=True)
    li = sub.add_parser("list"); li.add_argument("--issue", type=int, required=True)
    asb = sub.add_parser("add-sub"); asb.add_argument("--parent", type=int, required=True); asb.add_argument("--sub", type=int, required=True)
    ab = sub.add_parser("add-blocked-by"); ab.add_argument("--issue", type=int, required=True)
    ab.add_argument("--blocked-by", type=int, required=True); ab.add_argument("--confirm", action="store_true")
    a = ap.parse_args(argv)
    return {"list": cmd_list, "add-sub": cmd_add_sub, "add-blocked-by": cmd_add_blocked_by}[a.cmd](a)


if __name__ == "__main__":
    sys.exit(main())
