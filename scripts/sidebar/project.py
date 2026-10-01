"""Project sidebar tool: add/remove issue to/from project, read and set fields (esp. Status).

Usage examples:
  python scripts/sidebar/project.py get --issue 35
  python scripts/sidebar/project.py set-status --issue 35 --status "In progress"
  python scripts/sidebar/project.py add --issue 35
  python scripts/sidebar/project.py remove --issue 35

All commands print a single JSON object to stdout.
Exit code 0 on success (including already-correct no-op), 1 on failure with {"ok": false, "error": ...}.
"""
import argparse
import json
import subprocess
import sys
from pathlib import Path

CONFIG_PATH = Path(__file__).resolve().parents[2] / ".agent" / "project-config.json"


def load_config():
    with open(CONFIG_PATH, encoding="utf-8") as f:
        return json.load(f)


def run_gh(args):
    p = subprocess.run(["gh"] + args, capture_output=True, text=True,
                       encoding="utf-8", errors="replace")
    return p


def repo_owner_name():
    p = run_gh(["repo", "view", "--json", "nameWithOwner", "--jq", ".nameWithOwner"])
    if p.returncode == 0 and "/" in (p.stdout or ""):
        owner, name = p.stdout.strip().split("/", 1)
        return owner, name
    return None, None


def find_item_id(issue_number, project_number, owner):
    """Return project item ID for issue, or None (GraphQL exact lookup)."""
    ro, rn = repo_owner_name()
    if ro and rn:
        return find_item_id_graphql(issue_number, ro, rn)
    return None


def find_item_id_graphql(issue_number, repo_owner, repo_name):
    """Fallback lookup of the project item ID via GraphQL (exact)."""
    q = ('query($owner:String!,$repo:String!,$num:Int!) {'
         ' repository(owner:$owner,name:$repo) { issue(number:$num) {'
         ' projectItems(first:20) { nodes { id project { ... on ProjectV2 { title } } } } } } }')
    p = run_gh(["api", "graphql", "-f", f"query={q}",
                "-f", f"owner={repo_owner}", "-f", f"repo={repo_name}",
                "-F", f"num={issue_number}"])
    if p.returncode != 0:
        return None
    try:
        data = json.loads(p.stdout)
        nodes = data["data"]["repository"]["issue"]["projectItems"]["nodes"]
        cfg = load_config()
        for n in nodes:
            if n["project"]["title"] == cfg["project"]["title"]:
                return n["id"]
    except Exception:
        return None
    return None


def read_status(item_id):
    q = ('query($item:ID!) { node(id:$item) { ... on ProjectV2Item {'
         ' fieldValues(first:30) { nodes { ... on ProjectV2ItemFieldSingleSelectValue'
         ' { name field { ... on ProjectV2SingleSelectField { name } } } } } } } }')
    p = run_gh(["api", "graphql", "-f", f"query={q}", "-f", f"item={item_id}"])
    if p.returncode != 0:
        raise RuntimeError(f"status read failed: {p.stderr.strip()}")
    data = json.loads(p.stdout)
    for v in data["data"]["node"]["fieldValues"]["nodes"]:
        if v and v.get("field", {}).get("name") == "Status":
            return v.get("name")
    return None


def cmd_get(args):
    cfg = load_config()
    owner = cfg["project"]["owner"]
    proj_num = cfg["project"]["number"]
    item_id = find_item_id(args.issue, proj_num, owner)
    if item_id is None:
        # try GraphQL with repo from gh
        rp = run_gh(["repo", "view", "--json", "nameWithOwner", "--jq", ".nameWithOwner"])
        if rp.returncode == 0 and "/" in rp.stdout:
            ro, rn = rp.stdout.strip().split("/", 1)
            item_id = find_item_id_graphql(args.issue, ro, rn)
    if item_id is None:
        print(json.dumps({"ok": True, "issue": args.issue, "inProject": False,
                          "itemId": None, "status": None}))
        return 0
    status = read_status(item_id)
    print(json.dumps({"ok": True, "issue": args.issue, "inProject": True,
                      "itemId": item_id, "status": status}))
    return 0


def cmd_add(args):
    cfg = load_config()
    owner = cfg["project"]["owner"]
    proj_num = cfg["project"]["number"]
    rp = run_gh(["repo", "view", "--json", "nameWithOwner", "--jq", ".nameWithOwner"])
    repo = rp.stdout.strip() if rp.returncode == 0 else ""
    # check existing first (idempotent)
    item_id = find_item_id(args.issue, proj_num, owner)
    if item_id:
        print(json.dumps({"ok": True, "issue": args.issue, "action": "already_present",
                          "itemId": item_id, "status": read_status(item_id)}))
        return 0
    url = f"https://github.com/{repo}/issues/{args.issue}" if repo else str(args.issue)
    p = run_gh(["project", "item-add", str(proj_num), "--owner", owner, "--url", url])
    if p.returncode != 0:
        print(json.dumps({"ok": False, "error": p.stderr.strip() or p.stdout.strip()}))
        return 1
    # gh prints item ID; re-read to confirm
    item_id = find_item_id(args.issue, proj_num, owner)
    status = read_status(item_id) if item_id else None
    print(json.dumps({"ok": True, "issue": args.issue, "action": "added",
                      "itemId": item_id, "status": status}))
    return 0


def cmd_remove(args):
    cfg = load_config()
    owner = cfg["project"]["owner"]
    proj_num = cfg["project"]["number"]
    item_id = find_item_id(args.issue, proj_num, owner)
    if item_id is None:
        print(json.dumps({"ok": True, "issue": args.issue, "action": "already_absent"}))
        return 0
    if not args.confirm:
        print(json.dumps({"ok": False, "error": "bulk/removal needs --confirm; showing plan only",
                          "plan": {"issue": args.issue, "itemId": item_id}}))
        return 1
    p = run_gh(["project", "item-delete", str(proj_num), "--owner", owner, "--id", item_id])
    if p.returncode != 0:
        print(json.dumps({"ok": False, "error": p.stderr.strip()}))
        return 1
    print(json.dumps({"ok": True, "issue": args.issue, "action": "removed", "itemId": item_id}))
    return 0


def set_status_once(cfg, item_id, status_name):
    opts = cfg["options"]
    if status_name not in opts:
        return False, f"unknown Status '{status_name}'; known: {sorted(opts)}"
    p = run_gh(["project", "item-edit", "--project-id", cfg["project"]["nodeId"],
                "--id", item_id, "--field-id", cfg["statusField"]["id"],
                "--single-select-option-id", opts[status_name]])
    if p.returncode != 0:
        return False, p.stderr.strip() or p.stdout.strip()
    return True, ""


def cmd_set_status(args):
    cfg = load_config()
    owner = cfg["project"]["owner"]
    proj_num = cfg["project"]["number"]
    item_id = find_item_id(args.issue, proj_num, owner)
    if item_id is None:
        # add first, then set
        rc = cmd_add(args)
        if rc != 0:
            return rc
        item_id = find_item_id(args.issue, proj_num, owner)
        if item_id is None:
            print(json.dumps({"ok": False, "error": "added but item ID not found on re-read"}))
            return 1
    current = read_status(item_id)
    if current == args.status:
        print(json.dumps({"ok": True, "issue": args.issue, "itemId": item_id,
                          "action": "already_set", "status": current}))
        return 0
    ok, err = set_status_once(cfg, item_id, args.status)
    if not ok:
        print(json.dumps({"ok": False, "issue": args.issue, "itemId": item_id, "error": err}))
        return 1
    # verify by re-reading; retry once if unchanged
    verified = read_status(item_id)
    if verified != args.status:
        ok2, err2 = set_status_once(cfg, item_id, args.status)
        if not ok2:
            print(json.dumps({"ok": False, "issue": args.issue, "itemId": item_id,
                              "error": err2, "previous": current, "readBack": verified}))
            return 1
        verified = read_status(item_id)
        if verified != args.status:
            print(json.dumps({"ok": False, "issue": args.issue, "itemId": item_id,
                              "error": f"set attempted twice but read-back is '{verified}'",
                              "previous": current, "readBack": verified}))
            return 1
    print(json.dumps({"ok": True, "issue": args.issue, "itemId": item_id,
                      "action": "updated", "previous": current, "status": verified}))
    return 0


def main(argv=None):
    ap = argparse.ArgumentParser(description="Project sidebar tool")
    sub = ap.add_subparsers(dest="cmd", required=True)
    g = sub.add_parser("get", help="read project membership and Status")
    g.add_argument("--issue", type=int, required=True)
    s = sub.add_parser("set-status", help="set Status with verify+retry")
    s.add_argument("--issue", type=int, required=True)
    s.add_argument("--status", required=True,
                   help="Todo | Needs your answer | In progress | In review | Needs correction | Done")
    a = sub.add_parser("add", help="add issue to project")
    a.add_argument("--issue", type=int, required=True)
    r = sub.add_parser("remove", help="remove issue from project (needs --confirm)")
    r.add_argument("--issue", type=int, required=True)
    r.add_argument("--confirm", action="store_true")
    args = ap.parse_args(argv)
    try:
        if args.cmd == "get":
            return cmd_get(args)
        if args.cmd == "add":
            return cmd_add(args)
        if args.cmd == "remove":
            return cmd_remove(args)
        if args.cmd == "set-status":
            return cmd_set_status(args)
    except RuntimeError as e:
        print(json.dumps({"ok": False, "error": str(e)}))
        return 1
    return 1


if __name__ == "__main__":
    sys.exit(main())
