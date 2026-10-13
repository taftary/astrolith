"""Where validation evidence lives, whichever checkout runs (#613).

Untracked evidence (timestamped run folders: logs, screenshots, report and
verdict files) is written to and read from the *main* checkout's
`.agent/validation/`, so a worktree can be removed after its merge without
losing anything. Tracked files (each issue's `requirements.md`,
`known-misses.md`) stay in the checkout being validated and travel with its
branch. When the owner accepts an issue (Done), its untracked evidence is
deleted (`clean_issue`); `scripts/sidebar/cleanup_evidence.py` does the same
for older evidence in bulk. Tracked files are never deleted here.
"""
import re
import shutil
import subprocess
from pathlib import Path

RUN = re.compile(r"^20\d{6}T\d{6}Z$")


def main_checkout(root):
    """The main checkout for the checkout at `root` (itself when it is one)."""
    p = subprocess.run(["git", "-C", str(root), "rev-parse", "--path-format=absolute",
                        "--git-common-dir"], capture_output=True, text=True,
                       encoding="utf-8", errors="replace")
    common = (p.stdout or "").strip()
    if p.returncode != 0 or not common:
        return Path(root)
    common = Path(common)
    return common.parent if common.name == ".git" else common


def evidence_dir(root):
    """The main checkout's `.agent/validation/`, for untracked evidence."""
    return main_checkout(root) / ".agent" / "validation"


def tracked(main, folder):
    """Repo-relative paths git tracks under `folder` in the main checkout."""
    rel = Path(folder).resolve().relative_to(Path(main).resolve()).as_posix()
    p = subprocess.run(["git", "-C", str(main), "ls-files", "--", rel],
                       capture_output=True, text=True, encoding="utf-8", errors="replace")
    return {line.strip() for line in (p.stdout or "").splitlines() if line.strip()}


def runs(issue_dir):
    """Timestamped run folders of one issue, oldest first."""
    d = Path(issue_dir)
    if not d.is_dir():
        return []
    return sorted(child for child in d.iterdir() if child.is_dir() and RUN.match(child.name))


def size(path):
    """Bytes under `path`."""
    return sum(f.stat().st_size for f in Path(path).rglob("*") if f.is_file())


def untracked_children(root, issue):
    """Everything under an issue's evidence folder that git does not track
    (run folders, screenshot folders, loose files); tracked ones are kept."""
    main = main_checkout(root)
    d = evidence_dir(root) / f"issue-{issue}"
    if not d.is_dir():
        return []
    keep = tracked(main, d)
    out = []
    for child in sorted(d.iterdir()):
        rel = child.resolve().relative_to(Path(main).resolve()).as_posix()
        holds_tracked = any(path == rel or path.startswith(rel + "/") for path in keep)
        if not holds_tracked:
            out.append(child)
    return out


def clean_issue(root, issue):
    """Deletes an accepted issue's untracked evidence; returns what went."""
    removed = []
    for child in untracked_children(root, issue):
        if child.is_dir():
            shutil.rmtree(child, ignore_errors=True)
        else:
            child.unlink(missing_ok=True)
        removed.append(child)
    return removed
