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
import time
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


# Forbidden shapes for validator criteria (issue #92): a criterion that asserts
# a pull-request run observation (step/check conclusion, annotation, CI
# greenness) is unsatisfiable by construction, because the run cannot go green
# until the verdict the MET n/n count is built from exists. Such observations
# are gate-owned (merge_gate.py + verdict comment) and must stay out of the
# criteria file. Only criterion lines are scanned, never prose.
SHAPE_PATTERNS = [
    r"conclusion is",
    r"is (not )?skipped",
    r"check-run",
    r"\bannotations?\b",
    r"run is green",
    r"green (at|on)",
    r"ci green",
    r"step conclusion",
]


def check_criteria_shape(text):
    """Return [(criterion_id, matched_phrase)] for unsatisfiable criteria."""
    crit_lines = []
    for line in text.splitlines():
        m = re.match(r"\s*-\s*\[\s\]\s*(C\d+):\s*(.*)", line)
        if m:
            crit_lines.append((m.group(1), m.group(2)))
    hits = []
    for cid, body in crit_lines:
        for pat in SHAPE_PATTERNS:
            m = re.search(pat, body, re.IGNORECASE)
            if m:
                hits.append((cid, m.group(0)))
                break
    return hits


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
    ap.add_argument("--issue", type=int, required=False, default=None)
    ap.add_argument("--sha", default="")
    ap.add_argument("--visual", default=None, choices=["yes", "no"],
                    help="what the approved spec claims (required with --issue): "
                         "runs frame_proof.py and maps it into the verdict")
    ap.add_argument("--allow", action="append", default=[],
                    help="allowlisted error regex (repeatable)")
    ap.add_argument("--check-criteria", default="",
                    help="direct-run criteria-shape check on FILE (authoring-time feedback, no full run)")
    a = ap.parse_args(argv)
    allow_res = [re.compile(p) for p in a.allow]

    if a.check_criteria:
        # Standalone shape check (issue #92): no issue, no runtime, just the refusal.
        try:
            text = Path(a.check_criteria).read_text(encoding="utf-8")
        except OSError as e:
            print(f"criteria-shape: cannot read {a.check_criteria}: {e}")
            return 2
        hits = check_criteria_shape(text)
        if hits:
            print(f"criteria-shape: REFUSED {a.check_criteria}:")
            for cid, phrase in hits:
                print(f"  - {cid} is unsatisfiable by construction "
                      f"(run observation inside MET n/n): matched '{phrase}'")
            return 1
        print(f"criteria-shape: OK {a.check_criteria} ({len(hits)} hits)")
        return 0

    if a.issue is None:
        print("criteria-shape: --issue N is required (or use --check-criteria FILE)")
        return 2

    if a.visual is None:
        print("frame proof: --visual yes|no is required with --issue N "
              "(what the approved spec claims)")
        return 2

    ts = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    start_mono = time.monotonic()
    outdir = AGENT / f"issue-{a.issue}" / ts
    outdir.mkdir(parents=True, exist_ok=True)

    req_path, drafted = ensure_requirements(a.issue)
    crits, unconfirmed = parse_criteria(req_path)

    findings = []
    errors, warnings = [], []
    blocked_reasons = []

    # --- criteria-shape refusal (issue #92): unsatisfiable criteria fail the run ---
    try:
        shape_hits = check_criteria_shape(req_path.read_text(encoding="utf-8"))
    except OSError:
        shape_hits = []
    for cid, phrase in shape_hits:
        errors.append(f"criteria-shape: {cid} is unsatisfiable by construction "
                      f"(run observation inside MET n/n): matched '{phrase}'")
    if shape_hits:
        findings.append(f"criteria-shape: REFUSED ({len(shape_hits)} hit(s), see errors)")

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

    # --- frame proof (issue #134): visual evidence as a mandatory step ---
    fp_dir = outdir / "frame-proof"
    fp_sha = a.sha
    if not fp_sha:
        hr, hout, _ = run(["git", "rev-parse", "HEAD"], timeout=60)
        fp_sha = (hout or "").strip().split()[0] if hr == 0 and (hout or "").strip() else ""
    fp_line, fp_visual = "", ""
    fp_fragment = fp_dir / "frame-proof.md"
    if not fp_sha:
        blocked_reasons.append("frame proof could not run: HEAD unreadable")
        findings.append("frame proof: BLOCKED (HEAD unreadable)")
    else:
        rc_fp, out_fp, err_fp = run(
            [sys.executable, "scripts/validation/frame_proof.py",
             "--issue", str(a.issue), "--sha", fp_sha,
             "--visual", str(a.visual),
             "--out-dir", str(fp_dir)], timeout=3600)
        (outdir / "frame-proof-run.log").write_text(
            f"$ frame_proof.py --issue {a.issue} --sha {fp_sha} --visual {a.visual}\n"
            f"rc={rc_fp}\n---stdout---\n{out_fp}\n---stderr---\n{err_fp}",
            encoding="utf-8")
        fp_line = next((ln for ln in (out_fp or "").splitlines()
                        if ln.startswith("FRAME-PROOF-")), "")
        try:
            fp_text = fp_fragment.read_text(encoding="utf-8")
            fp_visual = next((ln for ln in fp_text.splitlines()
                              if ln.startswith("Visual:")), "")
        except OSError:
            fp_visual = ""
        if rc_fp == 0 and fp_line.startswith("FRAME-PROOF-OK"):
            findings.append(f"frame proof: {fp_line} {fp_visual}".rstrip())
        elif rc_fp == 1:
            errors.append(f"frame proof mismatch: {fp_line or out_fp.strip()[:200]}")
            findings.append(f"frame proof: MISMATCH {fp_visual}".rstrip())
        else:
            blocked_reasons.append(
                f"frame proof could not run: {fp_line or out_fp.strip()[:200] or 'no output'}")
            findings.append("frame proof: BLOCKED (see frame-proof-run.log)")

    # --- known-misses regression checks ---
    km = AGENT / "known-misses.md"
    findings.append(f"known-misses log present={km.exists()}; its checks are this script's steps 1-3")

    # --- requirements table ---
    # Criterion probes, two forms (read-only, worktree files only):
    #   "(verify: contains TEXT in LOG)" where LOG is cargo-test.log |
    #   verify.log | edge-verify-repeat.log | frame-proof-run.log |
    #   capture-after.log | capture-before.log (MET iff the runtime log
    #   actually contains TEXT);
    #   "(verify: file RELPATH contains TEXT)" where RELPATH stays inside
    #   the repo (MET iff the worktree file actually contains TEXT).
    # Criteria without a probe stay UNVERIFIABLE -- never MET from a summary claim.
    # Timings, run counts, tree comparisons, and cache locations are judged
    # by the validator from the elapsed-time line, run records, and fixture
    # output that this notion's own tooling writes; the script marks those
    # UNVERIFIABLE and they never count as MET from a summary claim.
    fp_run_text = ""
    cap_after_text = ""
    cap_before_text = ""
    try:
        fp_run_text = (outdir / "frame-proof-run.log").read_text(encoding="utf-8")
    except OSError:
        fp_run_text = ""
    try:
        _fp_probe_dir = fp_dir if "fp_dir" in locals() else (outdir / "frame-proof")
        cap_after_text = (_fp_probe_dir / "capture-after.log").read_text(encoding="utf-8")
    except OSError:
        cap_after_text = ""
    try:
        _fp_probe_dir = fp_dir if "fp_dir" in locals() else (outdir / "frame-proof")
        cap_before_text = (_fp_probe_dir / "capture-before.log").read_text(encoding="utf-8")
    except OSError:
        cap_before_text = ""
    logs = {"cargo-test.log": out + "\n" + err, "verify.log": out2 + "\n" + err2,
            "edge-verify-repeat.log": out3 + "\n" + err3,
            "frame-proof-run.log": fp_run_text,
            "capture-after.log": cap_after_text,
            "capture-before.log": cap_before_text}
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
    try:
        _elapsed = int(time.monotonic() - start_mono)
    except (OSError, ValueError, NameError):
        _elapsed = -1
    if _elapsed >= 0:
        _mm, _ss = divmod(_elapsed, 60)
        findings.insert(0, f"elapsed: {_elapsed}s ({_mm:02d}m {_ss:02d}s wall clock; target under 300s)")
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
               "", "## Logs", "", "- cargo-test.log, verify.log, edge-verify-repeat.log in this folder",
               "- frame-proof/run: frame-proof-run.log, frame-proof/frame-proof.md (Issue verdict fragment), "
               "frame-proof/frame-proof.json, frame-proof/capture-after.log, frame-proof/capture-before.log"]
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
