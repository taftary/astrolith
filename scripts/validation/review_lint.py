"""Shape checker for the realism review document (issue #143, T1).

Stdlib only, read-only. Checks ``guides/universe/documents/realism-review.md`` (or
the path given as the single argument) against the shape fixed by
Spec v1 of #143:

- the twelve ``## N. Title`` headings, in order;
- section 3 has six ``###`` profile subsections, each with at least
  three findings ``- **F<letters><n>** ... [S<k>] ... Recommendation: ...``;
- section 4 has one table row per level L1-L11, each citing ``[S<k>]``;
- section 12 lists sources ``- **[S<k>]** Title — URL — read YYYY-MM-DD``,
  unique numbers, http(s) URLs, at least MIN_SOURCES of them;
- every ``[S<k>]`` cited anywhere exists in section 12.

Exit contract (stdout machine lines, no reserved vocabulary words):
  0  REVIEW-LINT-OK sources=<n> findings=<m>
  1  REVIEW-LINT-FAIL <first problem>
  2  REVIEW-LINT-BLOCKED <file could not be read>
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_PATH = ROOT / "guides" / "universe" / "documents" / "realism-review.md"

MIN_SOURCES = 20
PROFILE_COUNT = 6
MIN_FINDINGS_PER_PROFILE = 3
LEVELS = [f"L{n}" for n in range(1, 12)]

HEADINGS = [
    "## 1. How to read this",
    "## 2. Current state audit",
    "## 3. Panel findings",
    "## 4. Realism targets per level",
    "## 5. Themes",
    "## 6. Proposed ladder",
    "## 7. Loading and traversal design sketch",
    "## 8. Home path",
    "## 9. Proposed decisions (ADRs)",
    "## 10. Implementation order",
    "## 11. Hardware baseline",
    "## 12. Sources",
]

CITE_RE = re.compile(r"\[S(\d+)\]")
FINDING_RE = re.compile(r"^- \*\*F[A-Z]{1,2}\d+\*\*")
SOURCE_RE = re.compile(
    r"^- \*\*\[S(\d+)\]\*\* (?P<title>.+?) — (?P<url>\S+) — read (?P<date>\d{4}-\d{2}-\d{2})$"
)
TABLE_ROW_RE = re.compile(r"^\|")
TABLE_SEP_RE = re.compile(r"^\|\s*:?-{3,}")


def strip_fences(lines: list[str]) -> list[str]:
    """Blank out fenced code blocks so their text is never parsed."""
    out: list[str] = []
    fenced = False
    for line in lines:
        if line.strip().startswith("```"):
            fenced = not fenced
            out.append("")
            continue
        out.append("" if fenced else line)
    return out


def split_sections(lines: list[str]) -> tuple[list[str], dict[int, list[str]]]:
    """Return the ``## `` headings in order and the lines under each one.

    Sections are keyed by their 1-based position among the ``## `` headings.
    """
    headings: list[str] = []
    sections: dict[int, list[str]] = {}
    current = 0
    for line in lines:
        if line.startswith("## "):
            headings.append(line.rstrip())
            current += 1
            sections[current] = []
            continue
        if current:
            sections[current].append(line)
    return headings, sections


def lint(text: str) -> tuple[str | None, dict[str, int]]:
    """Return (first problem or None, counts)."""
    counts = {"sources": 0, "findings": 0}
    lines = strip_fences(text.splitlines())
    headings, sections = split_sections(lines)

    if headings != HEADINGS:
        for i, want in enumerate(HEADINGS):
            got = headings[i] if i < len(headings) else "(missing)"
            if got != want:
                return f"heading {i + 1} is {got!r}, want {want!r}", counts
        return f"{len(headings)} level-2 headings, want {len(HEADINGS)}", counts

    # Section 12: sources.
    sources: dict[int, str] = {}
    for line in sections[12]:
        if not line.startswith("- "):
            continue
        m = SOURCE_RE.match(line.rstrip())
        if not m:
            return f"source line does not match the required form: {line.strip()[:80]!r}", counts
        k = int(m.group(1))
        if k in sources:
            return f"source number S{k} is listed twice", counts
        url = m.group("url")
        if not (url.startswith("http://") or url.startswith("https://")):
            return f"source S{k} has no http(s) address", counts
        sources[k] = url
    counts["sources"] = len(sources)
    if len(sources) < MIN_SOURCES:
        return f"{len(sources)} sources, want at least {MIN_SOURCES}", counts

    # Section 3: profiles and findings.
    profiles: list[tuple[str, int]] = []
    name = None
    n = 0
    for line in sections[3]:
        if line.startswith("### "):
            if name is not None:
                profiles.append((name, n))
            name, n = line[4:].strip(), 0
            continue
        if FINDING_RE.match(line):
            if name is None:
                return f"finding outside a profile subsection: {line.strip()[:60]!r}", counts
            if not CITE_RE.search(line):
                return f"finding without a source number: {line.strip()[:60]!r}", counts
            if "Recommendation:" not in line:
                return f"finding without a recommendation: {line.strip()[:60]!r}", counts
            n += 1
            counts["findings"] += 1
    if name is not None:
        profiles.append((name, n))
    if len(profiles) != PROFILE_COUNT:
        return f"{len(profiles)} profile subsections in section 3, want {PROFILE_COUNT}", counts
    for pname, nfind in profiles:
        if nfind < MIN_FINDINGS_PER_PROFILE:
            return f"profile {pname!r} has {nfind} findings, want at least {MIN_FINDINGS_PER_PROFILE}", counts

    # Section 4: targets table, one cited row per level.
    seen_levels: set[str] = set()
    header_done = False
    for line in sections[4]:
        if not TABLE_ROW_RE.match(line):
            continue
        if not header_done:
            header_done = True
            continue
        if TABLE_SEP_RE.match(line):
            continue
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        level = cells[0] if cells else ""
        if level in LEVELS:
            if not CITE_RE.search(line):
                return f"targets row {level} has no source number", counts
            seen_levels.add(level)
    missing = [lv for lv in LEVELS if lv not in seen_levels]
    if missing:
        return f"targets table lacks rows for {', '.join(missing)}", counts

    # Every citation anywhere resolves.
    for i, line in enumerate(lines, 1):
        for m in CITE_RE.finditer(line):
            k = int(m.group(1))
            if k not in sources:
                return f"line {i} cites S{k}, which is not in section 12", counts

    return None, counts


def main(argv: list[str] | None = None) -> int:
    args = sys.argv[1:] if argv is None else argv
    path = Path(args[0]) if args else DEFAULT_PATH
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as exc:
        print(f"REVIEW-LINT-BLOCKED cannot read {path}: {exc.__class__.__name__}")
        return 2
    problem, counts = lint(text)
    if problem:
        print(f"REVIEW-LINT-FAIL {problem}")
        return 1
    print(f"REVIEW-LINT-OK sources={counts['sources']} findings={counts['findings']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
