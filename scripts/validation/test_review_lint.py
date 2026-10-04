"""Fixture harness for review_lint.py (issue #143, T1).

In-memory documents only, no network, no repository file read: a
passing fixture, then one failing fixture per rule (missing heading,
finding without source, dangling source number, too few sources, missing
level row, too few findings in a profile), plus the CLI exit codes.

Usage:
  python scripts/validation/test_review_lint.py
Exit 0 when every fixture behaves; non-zero naming the first failure.
"""

from __future__ import annotations

import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from review_lint import HEADINGS, MIN_SOURCES, lint, main as rl_main  # noqa: E402

PROFILES = ["Astronomer", "Astrophysicist", "Planetary scientist",
            "Mathematician", "Artist", "Engineer"]
LETTERS = ["A", "S", "G", "M", "R", "E"]


def check(name, got, want):
    if got != want:
        print(f"fixture {name}: MISMATCH (got {got!r}, want {want!r})")
        return False
    print(f"fixture {name}: ok")
    return True


def build(*, sources=MIN_SOURCES, drop_heading=None, finding_without_source=False,
          dangling=False, drop_level=None, short_profile=None) -> str:
    out: list[str] = []
    for idx, h in enumerate(HEADINGS, 1):
        if idx == drop_heading:
            continue
        out.append(h)
        out.append("")
        if idx == 3:
            for p, letter in zip(PROFILES, LETTERS):
                out.append(f"### {p}")
                count = 2 if p == short_profile else 3
                for n in range(1, count + 1):
                    cite = "" if (finding_without_source and n == 1 and letter == "A") else " [S1]"
                    out.append(f"- **F{letter}{n}** Something real{cite}. Recommendation: do it.")
                out.append("")
        if idx == 4:
            out.append("| Level | Quantity | Target | Today | Gap |")
            out.append("|---|---|---|---|---|")
            for n in range(1, 12):
                if n == drop_level:
                    continue
                out.append(f"| L{n} | things | 42 [S2] | 7 | big |")
            out.append("")
        if idx == 7 and dangling:
            out.append("See [S999] for the budget.")
            out.append("")
        if idx == 12:
            for k in range(1, sources + 1):
                out.append(f"- **[S{k}]** Title {k} — https://example.org/{k} — read 2026-10-04")
            out.append("")
    out.append("```")
    out.append("## 99. not a heading, inside a fence")
    out.append("```")
    return "\n".join(out) + "\n"


def main() -> int:
    ok = True
    problem, counts = lint(build())
    ok &= check("pass-problem", problem, None)
    ok &= check("pass-counts", counts, {"sources": MIN_SOURCES, "findings": 18})

    problem, _ = lint(build(drop_heading=6))
    ok &= check("missing-heading", problem is not None and problem.startswith("heading 6 is"), True)

    problem, _ = lint(build(finding_without_source=True))
    ok &= check("finding-no-source", problem is not None and "without a source" in problem, True)

    problem, _ = lint(build(dangling=True))
    ok &= check("dangling-source", problem is not None and "S999" in problem, True)

    problem, _ = lint(build(sources=MIN_SOURCES - 1))
    ok &= check("too-few-sources", problem, f"{MIN_SOURCES - 1} sources, want at least {MIN_SOURCES}")

    problem, _ = lint(build(drop_level=9))
    ok &= check("missing-level", problem, "targets table lacks rows for L9")

    problem, _ = lint(build(short_profile="Artist"))
    ok &= check("short-profile", problem, "profile 'Artist' has 2 findings, want at least 3")

    with tempfile.TemporaryDirectory(prefix="review-lint-test-") as d:
        good = Path(d) / "good.md"
        good.write_text(build(), encoding="utf-8")
        bad = Path(d) / "bad.md"
        bad.write_text(build(sources=3), encoding="utf-8")
        ok &= check("cli-ok-rc", rl_main([str(good)]), 0)
        ok &= check("cli-fail-rc", rl_main([str(bad)]), 1)
        ok &= check("cli-blocked-rc", rl_main([str(Path(d) / "missing.md")]), 2)
        p = subprocess.run([sys.executable, str(HERE / "review_lint.py"), str(good)],
                           capture_output=True, text=True, cwd=str(HERE.parent.parent))
        ok &= check("cli-ok-line", p.stdout.strip(), f"REVIEW-LINT-OK sources={MIN_SOURCES} findings=18")
        blob = p.stdout.lower()
        for word in ("error", "warn", "panic", "deprecated", "failed"):
            ok &= check(f"vocab-{word}", word not in blob, True)

    if not ok:
        print("fixtures: MISMATCH")
        return 1
    print("fixtures: all ok (review lint)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
