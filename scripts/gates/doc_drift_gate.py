"""Doc drift guard: process docs, instruction files, and rule references.

Checks (a)-(d) are the original CI inline checks, unchanged. Checks (e)-(k)
are the M1 extensions from Issue #85 (notion 4.5, 4.6 item 5): section list,
E- ID existence, pointer-only files, doc index, ADR naming, AGENTS.md
references, and .rs file-size warn/fail.

Exit 0 when everything passes (warnings for oversized files below the fail
threshold still exit 0). Exit 1 with a sorted failure list otherwise.
"""

import json
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent.parent

errors = []
warnings = []


def fail(message):
    errors.append(message)


# (a) No machine-usable status literals or option IDs outside the config.
cfg = json.loads((ROOT / ".agent/project-config.json").read_text(encoding="utf-8"))
option_ids = set(cfg.get("options", {}).values())
for path in list((ROOT / ".opencode/skills").rglob("SKILL.md")) + list(
    (ROOT / "scripts").rglob("*.py")
):
    text = path.read_text(encoding="utf-8")
    if re.search(r'--status\s+"[^"]+"', text):
        fail(f"{path.relative_to(ROOT)}: literal --status name; use --status-key")
    for oid in option_ids:
        if oid in text:
            fail(
                f"{path.relative_to(ROOT)}: hardcoded option ID {oid}; "
                "read it from .agent/project-config.json"
            )

# (b) Repository paths referenced from skills must exist.
for path in sorted((ROOT / ".opencode/skills").rglob("SKILL.md")):
    text = path.read_text(encoding="utf-8")
    for m in re.finditer(
        r"`((?:docs|scripts|drafts|crates|\.agent|\.github|tmp)/[A-Za-z0-9_.\-+/]*)`", text
    ):
        ref = m.group(1).rstrip("/")
        if ref.endswith((".md", ".py", ".yml", ".json", ".jsonc", ".toml")) or "/" in ref:
            if not (ROOT / ref).exists():
                fail(f"{path.relative_to(ROOT)}: referenced path does not exist: {ref}")

# (c) Every skill directory is named in docs/workflow.md and AGENTS.md.
skill_ids = sorted(p.name for p in (ROOT / ".opencode/skills").iterdir() if p.is_dir())
for doc in ("docs/workflow.md", "AGENTS.md"):
    text = (ROOT / doc).read_text(encoding="utf-8")
    for sid in skill_ids:
        if sid not in text:
            fail(f"{doc}: skill `{sid}` is not named")

# (d) Relative Markdown links must resolve.
roots = [ROOT / "docs", ROOT]
md_files = (
    [p for r in roots for p in r.glob("*.md")]
    + list((ROOT / "docs").rglob("*.md"))
    + [ROOT / "AGENTS.md", ROOT / "CLAUDE.md", ROOT / "README.md"]
)
seen = set()
for md in md_files:
    if md in seen or not md.is_file():
        continue
    seen.add(md)
    text = md.read_text(encoding="utf-8")
    for m in re.finditer(r"\]\(([^)#\s][^)\s]*)\)", text):
        target = m.group(1)
        if re.match(r"^(?:https?://|mailto:|#)", target):
            continue
        if (md.parent / target).exists():
            continue
        fail(f"{md.relative_to(ROOT)}: broken relative link: {target}")

# (e) docs/engineering.md contains every heading of the fixed section list.
FIXED_SECTIONS = [
    "Toolchain and pins",
    "Workspace layout",
    "Crate rules",
    "Module and file rules",
    "Naming, visibility, and API design",
    "Documentation",
    "Testing ladder and techniques",
    "Cargo features policy",
    "Error handling and diagnostics",
    "Headless protocol",
    "Numerics and determinism",
    "Performance and profiling",
    "Lint and format policy",
    "Dependencies and supply chain",
    "CI",
    "Adopt when needed",
    "How to add a crate, a plugin, a feature, a dependency, a decision",
    "References",
]
eng = (ROOT / "docs/engineering.md").read_text(encoding="utf-8")
eng_headings = set(re.findall(r"^## (.+)$", eng, re.MULTILINE))
for section in FIXED_SECTIONS:
    if section not in eng_headings:
        fail(f"docs/engineering.md: missing fixed section: {section}")

# (f) Every E- ID cited anywhere in the repository exists in engineering.md.
defined_ids = set(re.findall(r"\bE-[A-Z0-9]+(?:-[A-Z0-9]+)+\b", eng))
try:
    tracked = subprocess.run(
        ["git", "ls-files"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=True,
    ).stdout.splitlines()
except Exception:
    tracked = None
if tracked is None:
    candidates = [
        p
        for p in ROOT.rglob("*")
        if p.is_file()
        and ".git" not in p.parts
        and "target" not in p.parts
        and "tmp" not in p.parts
        and ".agent/validation" not in p.as_posix()
    ]
    tracked = [p.relative_to(ROOT).as_posix() for p in candidates]
id_pattern = re.compile(r"\bE-[A-Z0-9]+(?:-[A-Z0-9]+)+\b")
for rel in tracked:
    if rel == "docs/engineering.md":
        continue
    p = ROOT / rel
    if not p.is_file():
        continue
    if p.suffix not in (".md", ".rs", ".toml", ".py", ".yml", ".json", ".jsonc"):
        continue
    try:
        text = p.read_text(encoding="utf-8")
    except (UnicodeDecodeError, OSError):
        continue
    for cited in sorted(set(id_pattern.findall(text))):
        if cited not in defined_ids:
            fail(f"{rel}: cites unknown rule ID {cited}")

# (g) CLAUDE.md and CONTRIBUTING.md are pointer-only.
for pointer_file in ("CLAUDE.md", "CONTRIBUTING.md"):
    lines = (ROOT / pointer_file).read_text(encoding="utf-8").splitlines()
    if len(lines) > 8:
        fail(f"{pointer_file}: pointer-only file has {len(lines)} lines, at most eight")
    for i, line in enumerate(lines, start=1):
        if not line.strip() or line.startswith("#"):
            continue
        if "`" not in line and "http" not in line and "](" not in line:
            fail(f"{pointer_file}:{i}: non-heading line without a path or link")

# (h) docs/README.md names every file and folder directly under docs/.
index_text = (ROOT / "docs/README.md").read_text(encoding="utf-8")
for child in sorted((ROOT / "docs").iterdir()):
    if child.name not in index_text:
        fail(f"docs/README.md: does not name {child.name}")

# (i) docs/decisions/*.md match NNNN-[a-z0-9-]+.md, unique and consecutive.
adrs = sorted((ROOT / "docs/decisions").glob("*.md"))
numbers = []
for adr in adrs:
    if not re.fullmatch(r"\d{4}-[a-z0-9-]+\.md", adr.name):
        fail(f"docs/decisions/{adr.name}: bad ADR name, want NNNN-[a-z0-9-]+.md")
        continue
    numbers.append(int(adr.name[:4]))
if numbers:
    if sorted(set(numbers)) != numbers:
        fail("docs/decisions: duplicate ADR numbers")
    if numbers != list(range(1, max(numbers) + 1)):
        fail(f"docs/decisions: ADR numbers not consecutive from 0001: {numbers}")

# (j) AGENTS.md names both documents.
agents = (ROOT / "AGENTS.md").read_text(encoding="utf-8")
for doc in ("docs/engineering.md", "docs/ARCHITECTURE.md"):
    if doc not in agents:
        fail(f"AGENTS.md: does not name {doc}")

# (k) .rs file-size guard: warn over warn_lines, fail over fail_lines.
size_cfg = cfg.get("fileSizeGuard", {})
warn_lines = int(size_cfg.get("warnLines", 800))
fail_lines = int(size_cfg.get("failLines", 1200))
for rs in sorted((ROOT / "crates").rglob("*.rs")):
    count = sum(1 for _ in rs.open(encoding="utf-8", errors="replace"))
    rel = rs.relative_to(ROOT).as_posix()
    if count > fail_lines:
        fail(f"{rel}: {count} lines, over the {fail_lines} fail threshold")
    elif count > warn_lines:
        warnings.append(f"{rel}: {count} lines, over the {warn_lines} warn threshold")

for w in sorted(set(warnings)):
    print(f"warning: {w}")
if errors:
    print("Doc drift guard failures:")
    for e in sorted(set(errors)):
        print(f"  {e}")
    sys.exit(1)
print("Doc drift guard passed.")
