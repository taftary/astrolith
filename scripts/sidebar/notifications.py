"""Notifications sidebar tool: subscription state/reason, subscribe/unsubscribe/ignore.

Examples:
  python scripts/sidebar/notifications.py show --issue 35
  python scripts/sidebar/notifications.py subscribe --issue 35
  python scripts/sidebar/notifications.py unsubscribe --issue 35
  python scripts/sidebar/notifications.py ignore --issue 35 --confirm

Uses GraphQL updateSubscription mutation (no notification spam: never comments).
JSON to stdout.
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


def node_and_sub(issue):
    r = repo()
    q = ('query($o:String!,$r:String!,$n:Int!) { repository(owner:$o,name:$r) {'
         ' issue(number:$n) { id viewerSubscription } } }')
    o, name = r.split("/", 1)
    p = run_gh(["api", "graphql", "-f", f"query={q}", "-f", f"o={o}",
                "-f", f"r={name}", "-F", f"n={issue}"])
    if p.returncode != 0:
        return None, p.stderr.strip()
    try:
        iss = json.loads(p.stdout)["data"]["repository"]["issue"]
        return iss, ""
    except Exception as e:
        return None, str(e)


def cmd_show(a):
    iss, err = node_and_sub(a.issue)
    if iss is None:
        print(json.dumps({"ok": False, "error": err}))
        return 1
    r = repo()
    print(json.dumps({"ok": True, "issue": a.issue,
                      "subscribed": iss.get("viewerSubscription"),
                      "customizeUrl": f"https://github.com/{r}/subscription",
                      "note": "SUBSCRIBED=notify all; UNSUBSCRIBED=default; IGNORED=never notify"}))
    return 0


def set_state(issue, state, need_confirm=False, confirmed=False):
    if need_confirm and not confirmed:
        print(json.dumps({"ok": False, "error": f"{state} needs --confirm",
                          "plan": {"issue": issue, "state": state}}))
        return 1
    iss, err = node_and_sub(issue)
    if iss is None:
        print(json.dumps({"ok": False, "error": err}))
        return 1
    if iss.get("viewerSubscription") == state:
        print(json.dumps({"ok": True, "action": "already_set", "issue": issue, "state": state}))
        return 0
    q = ('mutation($id:ID!,$st:SubscriptionState!) { updateSubscription('
         'input:{subscribableId:$id,state:$st}) { subscribable { ... on Issue { viewerSubscription } } } }')
    p = run_gh(["api", "graphql", "-f", f"query={q}", "-f", f"id={iss['id']}", "-f", f"st={state}"])
    if p.returncode != 0:
        print(json.dumps({"ok": False, "error": p.stderr.strip()[:500]}))
        return 1
    print(json.dumps({"ok": True, "action": "updated", "issue": issue,
                      "previous": iss.get("viewerSubscription"), "state": state}))
    return 0


def main(argv=None):
    ap = argparse.ArgumentParser(description="Notifications sidebar tool")
    sub = ap.add_subparsers(dest="cmd", required=True)
    for name in ("show", "subscribe", "unsubscribe"):
        p = sub.add_parser(name); p.add_argument("--issue", type=int, required=True)
    ig = sub.add_parser("ignore"); ig.add_argument("--issue", type=int, required=True)
    ig.add_argument("--confirm", action="store_true")
    a = ap.parse_args(argv)
    if a.cmd == "show":
        return cmd_show(a)
    if a.cmd == "subscribe":
        return set_state(a.issue, "SUBSCRIBED")
    if a.cmd == "unsubscribe":
        return set_state(a.issue, "UNSUBSCRIBED")
    if a.cmd == "ignore":
        return set_state(a.issue, "IGNORED", need_confirm=True, confirmed=a.confirm)
    return 1


if __name__ == "__main__":
    sys.exit(main())
