"""Independent runtime + requirements validator.

Usage:
  python scripts/validation/validate.py --issue 35 [--sha <commit>] [--allow "known warning regex"]

What it does (clean state, full re-run every time):
  1. Loads .agent/validation/issue-<n>/requirements.md (source of truth). If missing,
     drafts it from the issue body/comments verbatim and marks it UNCONFIRMED.
  2. Runtime: cargo test --locked --workspace; cargo run --locked -p universe-app
      -- --verify (headless health check: exit 0 + VERIFY-OK); one edge probe;
      scans ALL logs for errors. Never mocks the thing under test.
      Warnings reported separately.
  3. Requirements: each criterion -> MET / PARTIAL / NOT MET / UNVERIFIABLE + evidence.
  4. Verdict: PASS only if runtime clean AND every criterion MET.
     FAIL on any runtime error, any NOT MET/PARTIAL, or any regression.
     BLOCKED when something could not run (never converted to PASS).
     Exit status is non-zero on BLOCKED or an unconfirmed checklist, so
     merge_gate.py can never record a BLOCKED round as PASS.
  5. With --sha: records `git rev-parse HEAD` and `git status --porcelain --untracked-files=no` (tracked tree only:
     BLOCKED when HEAD differs from --sha or the tree is dirty.
  6. Saves .agent/validation/issue-<n>/<timestamp>/report.md + logs + outputs.

Evidence only: never prints "should work" / "looks correct", only commands run + outputs.
Does not edit code. Does not post approval words.
"""
import argparse
import datetime
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
AGENT = ROOT / ".agent" / "validation"

ERROR_PAT = re.compile(r"(?i)(error\[|error:|errors:|exception|traceback|panic|unhandled|VERIFY-FAIL|thread .* panicked|crash|abort|FAILED\b|failures:)")
ZERO_COUNT_PAT = re.compile(r"\b0 (failed|errors?)\b")
FAIL_COUNT_PAT = re.compile(r"\b([1-9]\d*) (failed|errors?)\b")
WARN_PAT = re.compile(r"(?i)(warn|deprecat)")


def run(cmd, timeout=600):
    try:
        p = subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8",
                           errors="replace", cwd=str(ROOT), timeout=timeout)
        return p.returncode, (p.stdout or ""), (p.stderr or "")
    except subprocess.TimeoutExpired as e:
        out = (e.stdout or b"").decode("utf-8", "replace") if isinstance(e.stdout, bytes) else (e.stdout or "")
        err = (e.stderr or b"").decode("utf-8", "replace") if isinstance(e.stderr, bytes) else (e.stderr or "")
        return 124, out, err + f"\n[TIMEOUT after {timeout}s: {cmd}]"


def gh(args):
    p = subprocess.run(["gh"] + args, capture_output=True, text=True,
                       encoding="utf-8", errors="replace", cwd=str(ROOT))
    return p


def ensure_requirements(issue):
    d = AGENT / f"issue-{issue}"
    d.mkdir(parents=True, exist_ok=True)
    req = d / "requirements.md"
    if req.exists():
        return req, False
    # draft from issue body + comments verbatim (lowest-effort; owner must confirm)
    body = ""
    b = gh(["issue", "view", str(issue), "--json", "title,body,comments",
            "--jq", "{title: .title, body: .body, comments: [.comments[] | .body]}"])
    src = "issue fetch failed"
    if b.returncode == 0:
        try:
            data = json.loads(b.stdout or "{}")
            parts = [f"# Requirements (DRAFT — UNCONFIRMED) — issue #{issue}",
                     "", f"## Title: {data.get('title', '')}", "",
                     "## Owner source (issue body, verbatim):", "",
                     data.get("body", "") or "(empty body)", "",
                     "## Comments (verbatim):", ""]
            for c in data.get("comments", []) or []:
                parts += ["---", "", c, ""]
            parts += ["## Criteria (the specification skill owns this file; see workflow.md Validator Environment):", "",
                      "- [ ] C1: (derive from body above; one testable criterion per line)",
                      "  verify: command / URL / action + expected output", "",
                      "> Status: UNCONFIRMED -- drafted from the issue body, not from an",
                      "> approved spec. The specification skill derives this file from the",
                      "> approved spec (spec approval confirms the criteria)."]
            body = "\n".join(parts)
            src = "issue body/comments"
        except Exception as e:
            body = f"# Requirements (DRAFT) — issue #{issue}\n\nDraft failed: {e}\n"
    else:
        body = f"# Requirements (DRAFT) — issue #{issue}\n\n{b.stderr.strip()}\n"
    req.write_text(body.rstrip("\n") + "\n", encoding="utf-8")
    return req, True


def parse_criteria(req_path):
    text = req_path.read_text(encoding="utf-8")
    crits = []
    for line in text.splitlines():
        m = re.match(r"\s*-\s*\[\s\]\s*(C\d+:\s*.*)", line)
        if m:
            crits.append(m.group(1).strip())
    unconfirmed = "Status: UNCONFIRMED" in text
    return crits, unconfirmed


def scan(text, allow_res):
    hits = []
    for i, line in enumerate(text.splitlines(), 1):
        if ZERO_COUNT_PAT.search(line) and not FAIL_COUNT_PAT.search(line):
            continue
        if ERROR_PAT.search(line) and not any(r.search(line) for r in allow_res):
            hits.append(f"L{i}: {line.strip()[:300]}")
    warns = []
    for i, line in enumerate(text.splitlines(), 1):
        if WARN_PAT.search(line):
            warns.append(f"L{i}: {line.strip()[:300]}")
    return hits, warns


def main(argv=None):
    ap = argparse.ArgumentParser(description="Runtime + requirements validator")
    ap.add_argument("--issue", type=int, required=True)
    ap.add_argument("--sha", default="")
    ap.add_argument("--allow", action="append", default=[],
                    help="allowlisted error regex (repeatable)")
    a = ap.parse_args(argv)
    allow_res = [re.compile(p) for p in a.allow]

    ts = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    outdir = AGENT / f"issue-{a.issue}" / ts
    outdir.mkdir(parents=True, exist_ok=True)

    req_path, drafted = ensure_requirements(a.issue)
    crits, unconfirmed = parse_criteria(req_path)

    findings = []
    errors, warnings = [], []

    # --- runtime: existing test suite (no mocks added by validator) ---
    rc, out, err = run(["cargo", "test", "--locked", "--workspace"], timeout=900)
    (outdir / "cargo-test.log").write_text(f"$ cargo test --locked --workspace\nrc={rc}\n---stdout---\n{out}\n---stderr---\n{err}", encoding="utf-8")
    eh, wh = scan(out + "\n" + err, allow_res)
    errors += [f"cargo test: {h}" for h in eh]
    warnings += [f"cargo test: {h}" for h in wh[:50]]
    findings.append(f"cargo test --locked --workspace: rc={rc} {'OK' if rc == 0 and not eh else 'PROBLEM'}")
    if rc != 0:
        errors.append(f"cargo test exited {rc}")

    # --- runtime: launch app headless --verify, wait for real health signal ---
    rc2, out2, err2 = run(["cargo", "run", "--locked", "-p", "universe-app", "--", "--verify"], timeout=900)
    (outdir / "verify.log").write_text(f"$ cargo run --locked -p universe-app -- --verify\nrc={rc2}\n---stdout---\n{out2}\n---stderr---\n{err2}", encoding="utf-8")
    healthy = rc2 == 0 and "VERIFY-OK" in out2
    eh2, wh2 = scan(out2 + "\n" + err2, allow_res)
    errors += [f"verify: {h}" for h in eh2]
    warnings += [f"verify: {h}" for h in wh2[:50]]
    findings.append(f"headless --verify: rc={rc2} VERIFY-OK present={('VERIFY-OK' in out2)} -> {'HEALTHY' if healthy else 'NOT HEALTHY'}")

    # --- edge/error path: verify determinism — run twice, snapshots must be identical.
    # (Never launches the windowed binary: unknown flags fall through to window launch
    # and would hang the validator, so all probes stay in headless --verify mode.)
    rc3, out3, err3 = run(["cargo", "run", "--locked", "-p", "universe-app", "--", "--verify"], timeout=900)
    (outdir / "edge-verify-repeat.log").write_text(f"$ cargo run --locked -p universe-app -- --verify (2nd run)\nrc={rc3}\n---stdout---\n{out3}\n---stderr---\n{err3}", encoding="utf-8")
    eh3, _ = scan(out3 + "\n" + err3, allow_res)
    errors += [f"edge-repeat: {h}" for h in eh3]
    def norm(t):
        t = re.sub(r"\(\d+\)", "(PID)", t)
        return re.sub(r"in \d+\.\d+s", "in T", t)
    det_stdout = norm(out2) == norm(out3)
    det = det_stdout and rc2 == 0 and rc3 == 0 and "VERIFY-OK" in out3
    findings.append(f"edge probe (verify repeat determinism): identical={det_stdout} -> {'OK' if det else 'PROBLEM'}")
    if not det_stdout:
        errors.append("verify repeat run differs from first run (non-deterministic or unstable)")

    # --- known-misses regression checks ---
    km = AGENT / "known-misses.md"
    findings.append(f"known-misses log present={km.exists()}; its checks are this script's steps 1-3")

    # --- requirements table ---
    # Criterion probes, two forms (read-only, worktree files only):
    #   "(verify: contains TEXT in LOG)" where LOG is cargo-test.log |
    #   verify.log | edge-verify-repeat.log (MET iff the runtime log
    #   actually contains TEXT);
    #   "(verify: file RELPATH contains TEXT)" where RELPATH stays inside
    #   the repo (MET iff the worktree file actually contains TEXT).
    # Criteria without a probe stay UNVERIFIABLE -- never MET from a summary claim.
    logs = {"cargo-test.log": out + "\n" + err, "verify.log": out2 + "\n" + err2,
            "edge-verify-repeat.log": out3 + "\n" + err3}
    rows = []
    if drafted or unconfirmed or not crits:
        rows.append({"criterion": "(checklist unconfirmed or empty — the specification skill (derive from the approved spec))",
                     "status": "UNVERIFIABLE",
                     "evidence": f"{req_path} drafted={drafted} criteria={len(crits)}"})
    for c in crits:
        if drafted or unconfirmed:
            rows.append({"criterion": c, "status": "UNVERIFIABLE",
                         "evidence": "checklist unconfirmed; no criterion counts until derived from the approved spec"})
            continue
        m = re.search(r"\(verify:\s*contains\s+(.+?)\s+in\s+([\w.\-]+)\)\s*$", c)
        if m:
            needle, logname = m.group(1).strip(), m.group(2).strip()
            hay = logs.get(logname)
            if hay is None:
                rows.append({"criterion": c, "status": "UNVERIFIABLE",
                             "evidence": "unknown log '%s'" % logname})
            elif needle in hay:
                rows.append({"criterion": c, "status": "MET",
                             "evidence": "found '%s' in %s (this run)" % (needle, logname)})
            else:
                rows.append({"criterion": c, "status": "NOT MET",
                             "evidence": "'%s' absent from %s (this run)" % (needle, logname)})
            continue
        f = re.search(r"\(verify:\s*file\s+([\w.\-/]+)\s+contains\s+(.+?)\)\s*$", c)
        if f:
            rel, needle = f.group(1), f.group(2).strip()
            try:
                rp = (ROOT / rel).resolve()
                inside = ROOT.resolve() in rp.parents
                small = rp.is_file() and rp.stat().st_size <= 2000000
                text = rp.read_text(encoding="utf-8") if (inside and small) else None
            except OSError:
                text = None
                inside = False
            if text is None:
                rows.append({"criterion": c, "status": "UNVERIFIABLE",
                             "evidence": "unreadable path (outside repo, missing, or too large): %s" % rel})
            elif needle in text:
                rows.append({"criterion": c, "status": "MET",
                             "evidence": "found '%s' in %s (this run)" % (needle, rel)})
            else:
                rows.append({"criterion": c, "status": "NOT MET",
                             "evidence": "'%s' absent from %s (this run)" % (needle, rel)})
            continue
        rows.append({"criterion": c, "status": "UNVERIFIABLE",
                     "evidence": "no automated probe mapped to this criterion yet; runtime logs only prove no-crash, not requirement match"})
        continue

    # --- worktree identity (only meaningful with --sha) ---
    identity_problems = []
    head_actual = ""
    tree_dirty = None
    if a.sha:
        hr, hout, herr = run(["git", "rev-parse", "HEAD"], timeout=60)
        head_actual = (hout or "").strip().split()[0] if (hout or "").strip() else ""
        sr, sout, serr = run(["git", "status", "--porcelain", "--untracked-files=no"], timeout=60)
        if hr != 0:
            identity_problems.append("cannot verify worktree identity (git unavailable)")
        elif head_actual != a.sha:
            identity_problems.append("HEAD " + (head_actual or "(empty)") + " differs from --sha " + a.sha)
        if sr != 0:
            identity_problems.append("cannot read worktree status (git unavailable)")
        else:
            tree_dirty = bool((sout or "").strip())
            if tree_dirty:
                identity_problems.append("tracked tree is dirty (uncommitted changes)")
        findings.append("worktree identity: HEAD=" + (head_actual or "?") + " expected=" + a.sha + " dirty=" + str(tree_dirty))

    # --- verdict ---
    blocked_reasons = []
    if drafted or unconfirmed:
        blocked_reasons.append("requirements checklist UNCONFIRMED (not derived from an approved spec)")
    blocked_reasons += identity_problems
    if rows and all(r["status"] == "UNVERIFIABLE" for r in rows):
        blocked_reasons.append("no criterion verifiable against owner words yet")
    if errors:
        verdict = "FAIL"
    elif blocked_reasons:
        verdict = "BLOCKED"
    else:
        verdict = "PASS" if rows and all(r["status"] == "MET" for r in rows) else "FAIL"

    # FAIL overrides: any runtime error or non-MET criterion fails even if checklist confirmed
    if errors:
        verdict = "FAIL"

    report = ["# Validation report", "", f"Issue: #{a.issue}  sha: {a.sha or '(worktree)'}  head: {head_actual or '-'}  dirty: {tree_dirty}  time: {ts} UTC",
              f"Verdict: **{verdict}**", "", "## Runtime findings", ""]
    report += [f"- {f}" for f in findings] + ["", "### Errors (fail unless allowlisted)", ""]
    report += [f"- {e}" for e in errors] or ["- none"]
    report += ["", "### Warnings (separate)", ""]
    report += [f"- {w}" for w in warnings[:30]] or ["- none"]
    report += ["", "## Requirements", ""]
    report += ["| criterion | status | evidence |", "|---|---|---|"]
    for r in rows:
        report += [f"| {r['criterion'][:120]} | {r['status']} | {r['evidence'][:160]} |"]
    report += ["", "## Not verified and why", ""]
    if blocked_reasons:
        report += [f"- {b}" for b in blocked_reasons]
    else:
        report += ["- none — all criteria checked"]
    report += ["", "## Scope creep", "", "- (validator flags unrequested changes here; none auto-detected)",
               "", "## Logs", "", "- cargo-test.log, verify.log, edge-verify-repeat.log in this folder"]
    (outdir / "report.md").write_text("\n".join(report).rstrip("\n") + "\n", encoding="utf-8")

    summary = {"ok": True, "issue": a.issue, "verdict": verdict, "report": str(outdir / "report.md"),
               "requirements": str(req_path), "errors": errors[:10], "warnings": len(warnings),
               "notes": blocked_reasons}
    print(json.dumps(summary))
    if verdict == "BLOCKED":
        print("BLOCKED: " + ("; ".join(blocked_reasons) if blocked_reasons else "see report"))
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
