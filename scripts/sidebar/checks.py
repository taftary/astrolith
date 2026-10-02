"""Checks sidebar tool: wait for PR checks with visible progress and timeout.

Examples:
  python scripts/sidebar/checks.py wait --pr 61 --timeout 600 --interval 15
  python scripts/sidebar/checks.py wait --sha 176799aece86e128f4a7b52351eea42d7ceff345 --timeout 60

Polls check-runs for the head SHA. Progress goes to stderr, final JSON to stdout.
Exit 0 when all checks succeed, 1 on failure or timeout. Designed to run in
the background (non-blocking): start it, continue other work, get notified.
"""
import argparse
import json
import subprocess
import sys
import time
from datetime import datetime, timezone


def run_gh(args):
    return subprocess.run(["gh"] + args, capture_output=True, text=True,
                          encoding="utf-8", errors="replace")


def repo():
    p = run_gh(["repo", "view", "--json", "nameWithOwner", "--jq", ".nameWithOwner"])
    return p.stdout.strip() if p.returncode == 0 else ""


def pr_head_sha(pr_number):
    r = repo()
    p = run_gh(["api", f"repos/{r}/pulls/{pr_number}", "--jq", ".head.sha"])
    if p.returncode != 0:
        return None, p.stderr.strip()
    sha = (p.stdout or "").strip()
    return (sha, "") if sha else (None, "empty head sha")


def check_runs(sha):
    r = repo()
    p = run_gh(["api", f"repos/{r}/commits/{sha}/check-runs?per_page=100"])
    if p.returncode != 0:
        return None, p.stderr.strip()
    try:
        data = json.loads(p.stdout or "{}")
    except json.JSONDecodeError as e:
        return None, str(e)
    runs = data.get("check_runs", []) if isinstance(data, dict) else []
    slim = [{"name": x.get("name"), "status": x.get("status"),
             "conclusion": x.get("conclusion")} for x in runs]
    return slim, ""


def summarize(runs):
    if not runs:
        return "pending", "no check runs yet"
    statuses = {x["status"] for x in runs}
    if statuses != {"completed"}:
        return "pending", f"{len(runs)} run(s), waiting: {sorted(statuses)}"
    conclusions = sorted({(x["conclusion"] or "unknown") for x in runs})
    if all(x["conclusion"] == "success" for x in runs):
        return "success", f"{len(runs)} run(s) success"
    return "failure", f"conclusions: {conclusions}"


def cmd_wait(a):
    if a.sha:
        sha = a.sha
        pr = a.pr
    elif a.pr:
        sha, err = pr_head_sha(a.pr)
        if not sha:
            print(json.dumps({"ok": False, "error": f"cannot resolve head SHA for PR #{a.pr}: {err}"}))
            return 1
        pr = a.pr
    else:
        print(json.dumps({"ok": False, "error": "need --pr or --sha"}))
        return 1
    deadline = time.time() + a.timeout
    attempt = 0
    while True:
        attempt += 1
        runs, err = check_runs(sha)
        if runs is None:
            print(f"[{datetime.now(timezone.utc).isoformat()}] poll {attempt}: api error: {err}",
                  file=sys.stderr)
        else:
            state, msg = summarize(runs)
            print(f"[{datetime.now(timezone.utc).isoformat()}] poll {attempt}: {state}: {msg}",
                  file=sys.stderr)
            if state == "success":
                print(json.dumps({"ok": True, "state": "success", "pr": pr,
                                  "sha": sha, "runs": runs, "attempts": attempt}))
                return 0
            if state == "failure":
                failed = [x["name"] for x in runs if x["conclusion"] not in ("success",)]
                print(json.dumps({"ok": False, "state": "failure", "pr": pr,
                                  "sha": sha, "runs": runs, "failed": failed,
                                  "attempts": attempt}))
                return 1
        if time.time() >= deadline:
            print(json.dumps({"ok": False, "state": "timeout", "pr": pr,
                              "sha": sha, "attempts": attempt,
                              "error": f"timed out after {a.timeout}s"}))
            return 1
        sleep_for = min(a.interval, max(1, int(deadline - time.time())))
        time.sleep(sleep_for)


def main(argv=None):
    ap = argparse.ArgumentParser(description="Checks sidebar tool")
    sub = ap.add_subparsers(dest="cmd", required=True)
    w = sub.add_parser("wait", help="wait for PR checks with progress + timeout")
    w.add_argument("--pr", type=int, default=None)
    w.add_argument("--sha", default="")
    w.add_argument("--timeout", type=int, default=600)
    w.add_argument("--interval", type=int, default=15)
    a = ap.parse_args(argv)
    return {"wait": cmd_wait}[a.cmd](a)


if __name__ == "__main__":
    sys.exit(main())
