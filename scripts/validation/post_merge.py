"""Post-merge validation: prove main holds the validated code (issue #198, AC3).

Usage:
  python scripts/validation/post_merge.py --issue N --validated <sha>

Compares the merged tree on main against the validated commit. When the
contents are identical it runs the headless app check once (`--verify`:
exit 0 + VERIFY-OK) and records the result; when they differ it falls
back to the full validation, so nothing is ever skipped by assumption.
Both SHAs are recorded on the Issue by the caller (the pull-request
skill posts the evidence naming the merge SHA).

Writes .agent/validation/issue-N/<timestamp>/post-merge.md with the
comparison, the verify output tail, and the run's own elapsed seconds.
Exit 0 when the record shows PASS (identical + VERIFY-OK, or the
fallback full validation passed); non-zero otherwise.
"""

import argparse
import datetime
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
AGENT = ROOT / ".agent" / "validation"


def run(cmd, timeout=900):
    try:
        p = subprocess.run(cmd, capture_output=True, text=True,
                           encoding="utf-8", errors="replace",
                           cwd=str(ROOT), timeout=timeout)
        return p.returncode, (p.stdout or ""), (p.stderr or "")
    except subprocess.TimeoutExpired as e:
        out = e.stdout if isinstance(e.stdout, str) else ""
        err = e.stderr if isinstance(e.stderr, str) else ""
        return 124, out, err + f"\n[TIMEOUT after {timeout}s]"


def main(argv=None):
    ap = argparse.ArgumentParser(description="Post-merge tree-identity proof")
    ap.add_argument("--issue", type=int, required=True)
    ap.add_argument("--validated", required=True,
                    help="full SHA of the validated (pre-merge) commit")
    a = ap.parse_args(argv)
    start = time.monotonic()
    ts = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    outdir = AGENT / f"issue-{a.issue}" / ts
    outdir.mkdir(parents=True, exist_ok=True)

    head = ""
    hr, hout, _ = run(["git", "rev-parse", "HEAD"], timeout=60)
    if hr == 0 and (hout or "").strip():
        head = hout.strip().split()[0]

    dr, diff_out, diff_err = run(["git", "diff", "--quiet",
                          a.validated, "HEAD"], timeout=120)
    identical = dr == 0 and bool(head) and head != ""
    diff_note = ("tree-identical" if identical
                 else f"tree-differ (git diff rc={dr})")

    verify_ok = False
    verify_tail = ""
    fallback = ""
    if identical:
        rc, out, err = run(["cargo", "run", "--locked", "-p", "universe-app",
                            "--", "--verify"], timeout=900)
        (outdir / "verify.log").write_text(
            f"$ cargo run --locked -p universe-app -- --verify\nrc={rc}\n"
            f"---stdout---\n{out}\n---stderr---\n{err}", encoding="utf-8")
        verify_ok = rc == 0 and "VERIFY-OK" in out
        tail = (out or "").strip().splitlines()
        verify_tail = tail[-1][:200] if tail else "(empty output)"
    else:
        rc, out, err = run([sys.executable, "scripts/validation/validate.py",
                            "--issue", str(a.issue), "--sha", head,
                            "--visual", "no"], timeout=3600)
        (outdir / "fallback-validate.log").write_text(
            f"$ validate.py --issue {a.issue} --sha {head} --visual no\n"
            f"rc={rc}\n---stdout---\n{out}\n---stderr---\n{err}",
            encoding="utf-8")
        fallback = f"fallback full validation rc={rc}"
        verify_ok = rc == 0

    elapsed = int(time.monotonic() - start)
    passed = verify_ok
    verdict = "PASS" if passed else "FAIL"
    lines = ["# Post-merge validation", "",
             f"Issue: #{a.issue}  validated: {a.validated}  "
             f"merge: {head or '?'}  time: {ts} UTC",
             f"post-merge {verdict}: {diff_note}; {a.validated} {head or ''}",
             f"elapsed: {elapsed}s", ""]
    if identical:
        lines += [f"headless --verify: {'VERIFY-OK' if verify_ok else 'NOT HEALTHY'}",
                  f"tail: {verify_tail}"]
    else:
        lines += [fallback or "fallback did not run"]
    lines += ["", "## Logs", "",
              "- verify.log (identical tree) or fallback-validate.log (differing tree)"]
    (outdir / "post-merge.md").write_text("\n".join(lines).rstrip("\n") + "\n",
                                          encoding="utf-8")
    print(f"post-merge {verdict}: {diff_note} "
          f"(validated {a.validated[:7] if len(a.validated) >= 7 else a.validated} "
          f"merge {(head[:7] if head else '?')} elapsed {elapsed}s)")
    return 0 if passed else 1


if __name__ == "__main__":
    sys.exit(main())
