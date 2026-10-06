"""Spec gate: exits 0 only when the Issue holds everything implementation needs.

Usage:
  python scripts/gates/spec_gate.py --issue N

Requires on the Issue:
  1. An owner approval comment: trimmed body exactly `approved`
     (or `owner-test-only approved`, which also waives rule 3).
  2. A spec comment containing all sections: goal, non-goals,
     acceptance criteria, test plan, testability needs.
  3. A preflight pass comment (or the owner-test-only approval).

Exit 0 when all hold; non-zero printing exactly what is missing.
The `implementation` skill calls this as step 1 and stops on non-zero.
"""

import argparse
import json
import re
import subprocess
import sys

SECTIONS = ["goal", "non-goals", "acceptance criteria", "test plan",
            "testability needs"]

SPEC_VERSION = re.compile(r"^[\ufeff\s]*##\s+Spec\s+v(\d+)\b",
                          re.IGNORECASE | re.MULTILINE)


def gh_issue(issue):
    p = subprocess.run(
        ["gh", "issue", "view", str(issue), "--json", "title,body,comments"],
        capture_output=True, text=True, encoding="utf-8", errors="replace")
    if p.returncode != 0:
        print(f"spec_gate: cannot read issue #{issue}: {p.stderr.strip()}")
        return None
    try:
        return json.loads(p.stdout or "{}")
    except json.JSONDecodeError as e:
        print(f"spec_gate: cannot parse issue #{issue}: {e}")
        return None


def comment_bodies(data):
    return [c.get("body", "") or ""
            for c in data.get("comments", []) or []]


def find_newest_spec(comments):
    """Newest `Spec v<k>` comment: highest k, ties to the latest comment.

    Returns (index, version) or (None, None) when no comment carries a
    spec version. `comments` is a list of bodies in chronological order.
    """
    best = (None, None)
    for i, body in enumerate(comments):
        versions = [int(v) for v in SPEC_VERSION.findall(body or "")]
        if not versions:
            continue
        top = max(versions)
        if best[0] is None or top > best[1] or top == best[1]:
            best = (i, top)
    return best


def approvals_after(comments, idx):
    """owner approvals strictly after comment index `idx`.

    `idx` None means no spec exists, so nothing can count as approving it.
    """
    if idx is None:
        return {"approved": False, "owner_test_only": False}
    later = [(c or "").strip() for c in comments[idx + 1:]]
    return {"approved": any(c == "approved" for c in later),
            "owner_test_only": any(c == "owner-test-only approved"
                                   for c in later)}


def spec_has_sections(body):
    low = (body or "").lower()
    return all(s in low for s in SECTIONS)


def capability_rows(capabilities_text):
    """Capability cells from the registry's capabilities table.

    Only the first table whose header names `capability` counts: rows
    are the `| ...` lines under that header up to the next prose line.
    The sandbox boundary (`Allowed`/`Refused`), `Traps`, and `Not
    available` tables come later and can never cover a need.
    """
    lines = (capabilities_text or "").splitlines()
    start = None
    for i, line in enumerate(lines):
        if not line.startswith("|"):
            continue
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if cells and cells[0].lower() == "capability":
            start = i
            break
    if start is None:
        return []
    rows = []
    for line in lines[start + 1:]:
        if not line.startswith("|"):
            break
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if not cells or not cells[0]:
            continue
        if set(cells[0]) <= set("-: "):
            continue
        rows.append(cells[0])
    return rows


def preflight_decision(needs, capabilities_text):
    """Skip-or-force for the testability preflight (issue #198, AC8).

    Returns (decision, uncovered): `skip` when every need names a
    substring found (case-insensitive) in some proven capability row,
    else `force` with the uncovered needs listed. An empty needs list
    never skips: there is nothing to cover, so the preflight runs.
    """
    rows = capability_rows(capabilities_text)
    lowered = [r.lower() for r in rows]
    uncovered = [n for n in (needs or [])
                 if not any(n.lower() in r for r in lowered)]
    if needs and not uncovered:
        return "skip", []
    return "force", (uncovered if needs else ["(no testability needs listed)"])


def validate_skip_record(record, capabilities_text, needs=None):
    """A preflight skip record is valid only when it names the file and
    at least one registry entry covering each claimed need.

    Returns (True, "") on a well-formed skip, else (False, reason). A
    bare claim that names no entry is refused, so a skip can never pass
    by assertion alone.
    """
    body = record or ""
    if "preflight skip" not in body.lower():
        return False, "no preflight skip statement"
    if "validator-capabilities.md" not in body:
        return False, "skip names no capabilities file"
    rows = capability_rows(capabilities_text)
    named = [r for r in rows if r.lower() in body.lower()]
    if not named:
        return False, "skip names no proven capability entry"
    if needs:
        decision, uncovered = preflight_decision(needs, capabilities_text)
        if decision != "skip":
            return False, "skip claimed but needs uncovered: " + ", ".join(uncovered)
    return True, ""


def preflight_satisfied(comments, capabilities_text=None):
    """Rule 3 for the gate: a pass comment, or a valid skip record.

    The loose pass proximity stays for genuine runs; a skip counts only
    when `validate_skip_record` accepts it against the registry, so a
    bare claim never opens the gate.
    """
    if any(re.search(r"preflight[\s\S]{0,40}pass", c, re.IGNORECASE)
           for c in comments):
        return True, "pass"
    if capabilities_text is None:
        try:
            from pathlib import Path as _Path
            capabilities_text = _Path(__file__).resolve().parents[2] / "docs" / "validator-capabilities.md"
            capabilities_text = capabilities_text.read_text(encoding="utf-8")
        except OSError:
            capabilities_text = ""
    for c in comments:
        ok, _ = validate_skip_record(c or "", capabilities_text)
        if ok:
            return True, "skip"
    return False, ""


def main(argv=None):
    ap = argparse.ArgumentParser(description="Spec gate for a parent Issue")
    ap.add_argument("--issue", type=int, required=True)
    a = ap.parse_args(argv)

    data = gh_issue(a.issue)
    if data is None:
        return 2
    comments = comment_bodies(data)
    missing = []

    # A new spec version voids the previous approval: only an approval
    # posted after the newest `Spec v<k>` comment counts (issue #91).
    spec_idx, spec_ver = find_newest_spec(comments)
    if spec_idx is None:
        missing.append("spec version comment (`## Spec v<k>`)")
    votes = approvals_after(comments, spec_idx)
    owner_test_only = votes["owner_test_only"]
    approved = votes["approved"]
    if not approved and not owner_test_only:
        if spec_idx is None:
            missing.append("owner approval comment (`approved` or `owner-test-only approved`)")
        else:
            missing.append(f"owner approval after newest spec (v{spec_ver})")

    if spec_idx is None:
        spec_ok = False
    else:
        spec_ok = spec_has_sections(comments[spec_idx])
    if not spec_ok:
        if spec_idx is None:
            missing.append("spec comment with sections: " + ", ".join(SECTIONS))
        else:
            missing.append(f"spec sections in newest spec (v{spec_ver}): "
                           + ", ".join(SECTIONS))

    if not owner_test_only:
        satisfied, _kind = preflight_satisfied(comments)
        if not satisfied:
            missing.append("preflight pass comment (or a valid preflight skip record naming docs/validator-capabilities.md entries)")

    if missing:
        print(f"spec_gate: BLOCKED for issue #{a.issue}; missing:")
        for m in missing:
            print(f"  - {m}")
        return 1
    kind = "owner-test-only" if owner_test_only else "approved"
    print(f"spec_gate: OPEN for issue #{a.issue} ({kind}, spec sections present"
          + (", preflight waived" if owner_test_only else ", preflight pass-or-skip present") + ")")
    return 0


if __name__ == "__main__":
    sys.exit(main())
