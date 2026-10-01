"""Milestone sidebar tool: show/set/change/clear/list/create.

Examples:
  python scripts/sidebar/milestone.py show --issue 35
  python scripts/sidebar/milestone.py list-open
  python scripts/sidebar/milestone.py set --issue 35 --title "M1"
  python scripts/sidebar/milestone.py clear --issue 35   # needs --confirm
  python scripts/sidebar/milestone.py ensure --title "M1"  # create if missing

JSON to stdout; exit 0 on success (incl. already-correct), 1 on failure.
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


def issue_json(number):
    p = run_gh(["issue", "view", str(number), "--json", "number,title,milestone"])
    if p.returncode != 0:
        return None, p.stderr.strip()
    return json.loads(p.stdout), ""


def cmd_show(a):
    data, err = issue_json(a.issue)
    if data is None:
        print(json.dumps({"ok": False, "error": err}))
        return 1
    m = data.get("milestone")
    print(json.dumps({"ok": True, "issue": a.issue,
                      "milestone": m["title"] if m else None,
                      "display": m["title"] if m else "No milestone"}))
    return 0


def cmd_list_open(a):
    r = repo()
    p = run_gh(["api", f"repos/{r}/milestones?state=open&per_page=100", "--jq",
                "[.[] | {number, title, open_issues, due_on}]"])
    if p.returncode != 0:
        print(json.dumps({"ok": False, "error": p.stderr.strip()}))
        return 1
    print(json.dumps({"ok": True, "milestones": json.loads(p.stdout or "[]")}))
    return 0


def cmd_set(a):
    data, err = issue_json(a.issue)
    if data is None:
        print(json.dumps({"ok": False, "error": err}))
        return 1
    cur = (data.get("milestone") or {}).get("title")
    if cur == a.title:
        print(json.dumps({"ok": True, "issue": a.issue, "action": "already_set",
                          "milestone": cur}))
        return 0
    p = run_gh(["issue", "edit", str(a.issue), "--milestone", a.title])
    if p.returncode != 0:
        print(json.dumps({"ok": False, "issue": a.issue, "error": p.stderr.strip(),
                          "hint": f"if milestone '{a.title}' is missing, run ensure first"}))
        return 1
    print(json.dumps({"ok": True, "issue": a.issue, "action": "updated",
                      "previous": cur, "milestone": a.title}))
    return 0


def cmd_clear(a):
    data, err = issue_json(a.issue)
    if data is None:
        print(json.dumps({"ok": False, "error": err}))
        return 1
    if not data.get("milestone"):
        print(json.dumps({"ok": True, "issue": a.issue, "action": "already_set",
                          "milestone": None, "display": "No milestone"}))
        return 0
    if not a.confirm:
        print(json.dumps({"ok": False, "error": "clear needs --confirm",
                          "plan": {"issue": a.issue,
                                   "milestone": data["milestone"]["title"]}}))
        return 1
    p = run_gh(["issue", "edit", str(a.issue), "--remove-milestone"])
    if p.returncode != 0:
        print(json.dumps({"ok": False, "error": p.stderr.strip()}))
        return 1
    print(json.dumps({"ok": True, "issue": a.issue, "action": "cleared",
                      "previous": data["milestone"]["title"]}))
    return 0


def cmd_ensure(a):
    r = repo()
    p = run_gh(["api", f"repos/{r}/milestones?state=all&per_page=100", "--jq",
                f'[.[] | select(.title == "{a.title}") | .number]'])
    if p.returncode != 0:
        print(json.dumps({"ok": False, "error": p.stderr.strip()}))
        return 1
    found = json.loads(p.stdout or "[]")
    if found:
        print(json.dumps({"ok": True, "action": "already_present", "title": a.title}))
        return 0
    c = run_gh(["api", f"repos/{r}/milestones", "-X", "POST",
                "-f", f"title={a.title}"] + (["-f", f"description={a.description}"] if a.description else []))
    if c.returncode != 0:
        print(json.dumps({"ok": False, "error": c.stderr.strip()}))
        return 1
    print(json.dumps({"ok": True, "action": "created", "title": a.title}))
    return 0


def main(argv=None):
    ap = argparse.ArgumentParser(description="Milestone sidebar tool")
    sub = ap.add_subparsers(dest="cmd", required=True)
    s = sub.add_parser("show"); s.add_argument("--issue", type=int, required=True)
    lo = sub.add_parser("list-open")
    st = sub.add_parser("set"); st.add_argument("--issue", type=int, required=True); st.add_argument("--title", required=True)
    cl = sub.add_parser("clear"); cl.add_argument("--issue", type=int, required=True); cl.add_argument("--confirm", action="store_true")
    en = sub.add_parser("ensure"); en.add_argument("--title", required=True); en.add_argument("--description", default="")
    a = ap.parse_args(argv)
    return {"show": cmd_show, "list-open": cmd_list_open, "set": cmd_set,
            "clear": cmd_clear, "ensure": cmd_ensure}[a.cmd](a)


if __name__ == "__main__":
    sys.exit(main())
