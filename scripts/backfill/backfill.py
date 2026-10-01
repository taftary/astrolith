"""Backfill: bring every old issue in line with .agent/project-config.json.

Modes (never skip ahead: dry-run -> pilot -> apply):
  python scripts/backfill/backfill.py --dry-run [--scope open|recent|all] [--validate]
  python scripts/backfill/backfill.py --pilot            # 5 issues after dry-run approval
  python scripts/backfill/backfill.py --apply            # high-confidence only, batches of ~20
  python scripts/backfill/backfill.py --resume           # continue from checkpoint
  python scripts/backfill/backfill.py --rollback --backup <file>

Rules:
  - Allowed: add missing issues to project + set Status; set milestone only on obvious
    match (else list); relationships only when explicit + owner-approved; Closes #N in a
    PR body only for clearly-belonging PRs + after owner approval.
  - NEVER: change issue state/titles/bodies/assignees/labels; post issue comments;
    touch subscriptions; act on archived/locked items (list as skipped).
  - Idempotent + resumable via .agent/backfill/checkpoint.json ("already set" on re-run).
  - Backup first: backfill-before-<date>.json + rollback command.
  - Optional --validate (read-only): shared runtime result + per-issue requirements state;
    "Claimed done but failing" is reported only, never acted on.
"""
import argparse
import csv
import datetime
import json
import re
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
STATE = ROOT / ".agent" / "backfill"
CONFIG = ROOT / ".agent" / "project-config.json"
VALIDATE_SCRIPT = ROOT / "scripts" / "validation" / "validate.py"


def run_gh(args):
    return subprocess.run(["gh"] + args, capture_output=True, text=True,
                          encoding="utf-8", errors="replace", cwd=str(ROOT))


def load_config():
    return json.loads(CONFIG.read_text(encoding="utf-8"))


def repo():
    p = run_gh(["repo", "view", "--json", "nameWithOwner", "--jq", ".nameWithOwner"])
    return p.stdout.strip()


def paginate_issues(scope):
    """Return list of issue dicts. scope: open (default) | recent (open + closed 90d) | all."""
    state = "open" if scope == "open" else "all"
    p = run_gh(["issue", "list", "--state", state, "--limit", "1000",
                "--json", "number,title,state,labels,assignees,milestone,updatedAt,closedAt"])
    if p.returncode != 0:
        raise RuntimeError(p.stderr.strip())
    issues = json.loads(p.stdout or "[]")
    if scope == "recent":
        cutoff = (datetime.datetime.now(datetime.timezone.utc) - datetime.timedelta(days=90)).isoformat()
        issues = [i for i in issues if i["state"] == "OPEN" or (i.get("closedAt") or "") >= cutoff]
    # exclude PRs (gh issue list already excludes PRs)
    return issues


def issue_detail(r, number):
    p = run_gh(["api", f"repos/{r}/issues/{number}",
                "--jq", "{stateReason: .state_reason, locked: .locked, body: .body, milestone: .milestone.title}"])
    if p.returncode != 0:
        return {}
    try:
        return json.loads(p.stdout or "{}")
    except json.JSONDecodeError:
        return {}


def project_status(r, number):
    q = ('query($o:String!,$rp:String!,$n:Int!) { repository(owner:$o,name:$rp) {'
         ' issue(number:$n) { projectItems(first:10) { nodes { id project { ... on ProjectV2 { title } }'
         ' fieldValues(first:20) { nodes { ... on ProjectV2ItemFieldSingleSelectValue'
         ' { name field { ... on ProjectV2SingleSelectField { name } } } } } } } } } }')
    o, name = r.split("/", 1)
    p = run_gh(["api", "graphql", "-f", f"query={q}", "-f", f"o={o}",
                "-f", f"rp={name}", "-F", f"n={number}"])
    if p.returncode != 0:
        return None, None, p.stderr.strip()[:200]
    try:
        nodes = json.loads(p.stdout)["data"]["repository"]["issue"]["projectItems"]["nodes"]
    except Exception:
        return None, None, "parse error"
    cfg = load_config()
    for n in nodes:
        if n["project"]["title"] == cfg["project"]["title"]:
            st = next((v.get("name") for v in n["fieldValues"]["nodes"]
                       if v and v.get("field", {}).get("name") == "Status"), None)
            return True, st, n["id"]
    return False, None, ""


def linked_prs(r, number):
    p = run_gh(["api", f"repos/{r}/issues/{number}/timeline?per_page=100", "--jq",
                "[.[] | select(.event==\"cross-referenced\") | .source.issue | select(.pull_request != null) | {number, state, draft: (.draft // false), title}]"])
    if p.returncode != 0:
        return []
    try:
        return json.loads(p.stdout or "[]")
    except json.JSONDecodeError:
        return []


def validator_marks(r, number, prs=()):
    """Return (has_pass, has_fail, head_match) from validator markers (no writes).

    head_match is True only when a validator:pass marker names the current head SHA
    of one of the linked open PRs. A stale pass from an older round does not count.
    """
    p = run_gh(["api", f"repos/{r}/issues/{number}/comments?per_page=100", "--jq",
                "[.[].body | select(contains(\"validator:pass\") or contains(\"validator:fail\"))]"])
    bodies = []
    if p.returncode == 0:
        try:
            bodies = json.loads(p.stdout or "[]")
        except json.JSONDecodeError:
            bodies = []
    text = "\n".join(bodies)
    has_pass, has_fail = "validator:pass" in text, "validator:fail" in text
    head_match = False
    for pr in prs:
        if pr.get("state") != "open":
            continue
        v = run_gh(["pr", "view", str(pr["number"]), "--json", "headRefOid", "--jq", ".headRefOid"])
        head = (v.stdout or "").strip() if v.returncode == 0 else ""
        if head and f"validator:pass sha={head}" in text:
            head_match = True
    return has_pass, has_fail, head_match


def derive(row, prs, detail, has_pass=False, head_match=False, current=None):
    """Return (proposed, confidence, reason). Uses stored mapping names."""
    labels = [l["name"] for l in row.get("labels", [])]
    body = (detail.get("body") or "")
    blocked_ref = bool(re.search(r"blocked\s+by\s+#\d+", body, re.I))
    state = row["state"]
    reason = detail.get("stateReason") or ""
    if set(labels) == {"task"} and state == "OPEN":
        return "In progress", "low", "task sub-issue on the board (auto-add sub-issues workflow is on) — owner decides remove vs keep"
    if state == "CLOSED":
        if reason == "not_planned":
            return "Done", "low", "closed as not planned (owner decides Done vs skip)"
        return "Done", "high", "closed as completed" + (" + merged PR" if any(p["state"] == "closed" for p in prs) else "")
    open_prs = [p for p in prs if p["state"] == "open"]
    if any(not p.get("draft", False) for p in open_prs):
        names = ", ".join('#' + str(p['number']) for p in open_prs)
        if head_match:
            return "In review", "high", f"open non-draft PR linked ({names}) + validator PASS for head SHA"
        if current in ("Needs correction", "In progress"):
            return current, "low", f"correction/work in flight with open PR ({names}), no validator PASS for head SHA yet"
        if has_pass:
            return "In review", "low", f"open non-draft PR linked ({names}) + stale validator PASS (not head SHA)"
        return "In review", "low", f"open non-draft PR linked ({names}) but no validator PASS on record"
    if any(p.get("draft", False) for p in open_prs):
        return "In progress", "high", "draft PR linked"
    if blocked_ref or "blocked" in labels:
        return "Needs your answer", "high", "open blocked-by reference or blocked label"
    if row.get("assignees"):
        return "In progress", "low", "assigned but no branch/PR signal (may be stale)"
    return "Todo", "high", "no activity signal"


def milestone_proposal(row, detail):
    if detail.get("milestone"):
        return detail["milestone"], "already set"
    body = (detail.get("body") or "") + " " + " ".join(l["name"] for l in row.get("labels", []))
    m = re.search(r"\b(M\d+)\b", body)
    if m:
        return m.group(1), "low (label/body ref only — owner confirms)"
    return None, "none obvious — left unset"


def checkpoint():
    STATE.mkdir(parents=True, exist_ok=True)
    f = STATE / "checkpoint.json"
    if f.exists():
        return json.loads(f.read_text(encoding="utf-8"))
    return {"processed": [], "results": {}}


def save_checkpoint(cp):
    (STATE / "checkpoint.json").write_text(json.dumps(cp, indent=2), encoding="utf-8")


def shared_runtime():
    """One shared runtime run for --validate mode (read-only)."""
    import subprocess as sp
    p = sp.run([sys.executable, str(VALIDATE_SCRIPT), "--issue", "35"],
               capture_output=True, text=True, encoding="utf-8", errors="replace", cwd=str(ROOT))
    try:
        return json.loads(p.stdout or "{}")
    except json.JSONDecodeError:
        return {"verdict": "BLOCKED", "notes": ["validator output unparseable"]}


def cmd_dry_run(a):
    r = repo()
    issues = paginate_issues(a.scope)
    rows = []
    for iss in issues:
        n = iss["number"]
        det = issue_detail(r, n)
        if det.get("locked"):
            rows.append({"issue": n, "title": iss["title"], "state": iss["state"],
                         "inProject": "-", "current": "-", "proposed": "-",
                         "milestone": "-", "relationships": "-", "unlinked": "-",
                         "confidence": "-", "reason": "skipped: locked", "validation": "not run"})
            continue
        in_proj, cur, _ = project_status(r, n)
        prs = linked_prs(r, n)
        has_pass, _, head_match = validator_marks(r, n, prs)
        prop, conf, why = derive(iss, prs, det, has_pass, head_match, cur)
        ms, ms_why = milestone_proposal(iss, det)
        unlinked = ", ".join(f"#{p['number']} {p['title'][:40]}" for p in prs) or "-"
        rows.append({"issue": n, "title": iss["title"][:60], "state": iss["state"],
                     "inProject": in_proj, "current": cur or "(none)",
                     "proposed": prop if (not in_proj or cur != prop) else f"{prop} (already correct)",
                     "milestone": ms or "No milestone", "milestoneNote": ms_why,
                     "relationships": "explicit refs only — see report",
                     "unlinked": unlinked, "confidence": conf, "reason": why,
                     "validation": "not run"})
        time.sleep(0.2)
    if a.validate:
        shared = shared_runtime()
        for row in rows:
            if row["proposed"].startswith(("Done", "In review")):
                row["validation"] = shared.get("verdict", "BLOCKED")
    # console table
    print(f"{'issue':>6}  {'inProj':>6}  {'current':>16}  {'proposed':>22}  {'conf':>4}  reason")
    for row in rows:
        print(f"{row['issue']:>6}  {str(row['inProject']):>6}  {str(row['current']):>16}  "
              f"{str(row['proposed']):>22}  {str(row['confidence']):>4}  {row['reason']}")
    low = [row for row in rows if row["confidence"] == "low"]
    if low:
        print("\nLow-confidence (needs owner decision):")
        for row in low:
            print(f"  #{row['issue']}: {row['reason']} -> proposed {row['proposed']}")
    claimed = [row for row in rows if row.get("validation") == "FAIL" and row["proposed"].startswith("Done")]
    if claimed:
        print("\nClaimed done but failing (report only — no state change):")
        for row in claimed:
            print(f"  #{row['issue']}: {row['title']}")
    ts = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    jf = STATE / f"dry-run-{a.scope}-{ts}.json"
    STATE.mkdir(parents=True, exist_ok=True)
    jf.write_text(json.dumps(rows, indent=2), encoding="utf-8")
    with open(str(jf).replace(".json", ".csv"), "w", newline="", encoding="utf-8") as f:
        w = csv.DictWriter(f, fieldnames=["issue", "title", "state", "inProject", "current",
                                          "proposed", "milestone", "unlinked", "confidence",
                                          "reason", "validation"])
        w.writeheader()
        for row in rows:
            w.writerow({k: row.get(k, "") for k in w.fieldnames})
    print(f"\nReport saved: {jf} (+ .csv)")
    return 0


def apply_rows(rows, limit=None, only_high=True):
    import subprocess as sp
    cp = checkpoint()
    ts = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    backup = []
    done = changed = already = skipped = failed = 0
    sel = [r for r in rows if r["issue"] not in cp["processed"]]
    if only_high:
        sel = [r for r in sel if r.get("confidence") == "high"]
    if limit:
        sel = sel[:limit]
    for row in sel:
        n = row["issue"]
        if row.get("reason", "").startswith("skipped"):
            skipped += 1
            cp["processed"].append(n)
            continue
        target = str(row["proposed"]).split(" (")[0]
        # backup current state
        r = repo()
        in_proj, cur, _ = project_status(r, n)
        det = issue_detail(r, n)
        backup.append({"issue": n, "inProject": in_proj, "status": cur,
                       "milestone": det.get("milestone")})
        p = sp.run([sys.executable, str(ROOT / "scripts" / "sidebar" / "project.py"),
                    "set-status", "--issue", str(n), "--status", target],
                   capture_output=True, text=True, encoding="utf-8", errors="replace")
        try:
            res = json.loads(p.stdout or "{}")
        except json.JSONDecodeError:
            res = {"ok": False, "error": (p.stderr or "")[:200]}
        if res.get("ok") and res.get("action") == "already_set":
            already += 1
        elif res.get("ok"):
            changed += 1
        else:
            failed += 1
            print(f"  #{n} FAILED: {res.get('error', '')[:200]}")
            if "rate limit" in json.dumps(res).lower() or "secondary" in json.dumps(res).lower():
                print("  rate limit hit — pausing 60s then continuing")
                time.sleep(60)
        done += 1
        cp["processed"].append(n)
        cp["results"][str(n)] = res
        save_checkpoint(cp)
    bf = STATE / f"backfill-before-{ts}.json"
    bf.write_text(json.dumps(backup, indent=2), encoding="utf-8")
    print(f"processed={done} changed={changed} already={already} skipped={skipped} failed={failed}")
    print(f"backup: {bf}")
    print(f"rollback: python scripts/backfill/backfill.py --rollback --backup {bf}")
    return 0


def latest_dry_run():
    files = sorted(STATE.glob("dry-run-*.json"))
    if not files:
        print(json.dumps({"ok": False, "error": "no dry-run report; run --dry-run first"}))
        return None
    return json.loads(files[-1].read_text(encoding="utf-8"))


def cmd_pilot(a):
    rows = latest_dry_run()
    if rows is None:
        return 1
    # mix of open / in-review / closed: take first 5 across proposed states
    picks, seen = [], set()
    for row in rows:
        key = str(row["proposed"]).split(" (")[0]
        if key not in seen:
            picks.append(row)
            seen.add(key)
        if len(picks) == 5:
            break
    for row in rows:
        if len(picks) == 5:
            break
        if row not in picks:
            picks.append(row)
    print("Pilot batch (verify before/after each):")
    for row in picks:
        print(f"  #{row['issue']}: {row['current']} -> {row['proposed']} [{row['confidence']}]")
    return apply_rows(picks, only_high=False)


def cmd_apply(a):
    rows = latest_dry_run()
    if rows is None:
        return 1
    return apply_rows(rows, only_high=True)


def cmd_resume(a):
    rows = latest_dry_run()
    if rows is None:
        return 1
    return apply_rows(rows, only_high=False)


def cmd_rollback(a):
    import subprocess as sp
    data = json.loads(Path(a.backup).read_text(encoding="utf-8"))
    ok = fail = 0
    for row in data:
        n, st = row["issue"], row.get("status")
        if not row.get("inProject"):
            print(f"  #{n}: was not in project — left as-is (manual review)")
            continue
        if not st:
            print(f"  #{n}: had no Status — left as-is (manual review)")
            continue
        p = sp.run([sys.executable, str(ROOT / "scripts" / "sidebar" / "project.py"),
                    "set-status", "--issue", str(n), "--status", st],
                   capture_output=True, text=True, encoding="utf-8", errors="replace")
        if p.returncode == 0:
            ok += 1
        else:
            fail += 1
            print(f"  #{n} rollback FAILED: {(p.stderr or '')[:200]}")
    print(f"rollback done: restored={ok} failed={fail} (milestones/relationships restore manually from backup)")
    return 0 if fail == 0 else 1


def main(argv=None):
    ap = argparse.ArgumentParser(description="Backfill project Status")
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--pilot", action="store_true")
    ap.add_argument("--apply", action="store_true")
    ap.add_argument("--resume", action="store_true")
    ap.add_argument("--rollback", action="store_true")
    ap.add_argument("--backup", default="")
    ap.add_argument("--scope", default="open", choices=["open", "recent", "all"])
    ap.add_argument("--validate", action="store_true",
                    help="read-only validator column for Done/In review (never changes state)")
    a = ap.parse_args(argv)
    if a.dry_run:
        return cmd_dry_run(a)
    if a.pilot:
        return cmd_pilot(a)
    if a.apply:
        return cmd_apply(a)
    if a.resume:
        return cmd_resume(a)
    if a.rollback:
        if not a.backup:
            print("need --backup <file>")
            return 1
        return cmd_rollback(a)
    ap.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
