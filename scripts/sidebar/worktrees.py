"""Worktree helper for parallel agents (#613).

Pure path and marker rules plus the few file-system steps the development
tool needs, each a small function the fixture tests replace:

- the work root: `../astrolith-work/` next to the main checkout;
- folder names: `<N>-<slug>` for an issue's worktree and `<N>-<slug>-check`
  for its validator worktree (`branch_folder` for branches without an issue);
- the owner's choice per piece of work, recorded on the issue as a marker
  comment `<!-- worktree:yes -->` or `<!-- worktree:no -->`;
- the free-space floor (15 GB) checked before a worktree is created;
- the worktree's own cargo config, which points `ASTROLITH_SETTINGS` at a
  settings file inside the worktree (the game reads it first);
- the Claude project-memory link, so a worktree's agent shares the main
  checkout's memory (a junction on Windows, a symlink elsewhere);
- the evidence root: the main checkout's `.agent/`, whichever worktree a
  script runs in (untracked evidence only; tracked files stay on the branch).
"""
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

# Free space a new worktree needs, in bytes (Spec v4 item 7).
FLOOR_BYTES = 15 * 1024 ** 3

# Marker comments recording the owner's choice (Spec v4 item 0).
MARKER = re.compile(r"<!--\s*worktree:(yes|no)\s*-->")

# Settings file a worktree's game uses, relative to the worktree.
SETTINGS_FILE = ".agent-local/settings.toml"

WORK_DIR_NAME = "astrolith-work"


def main_checkout(common_dir):
    """The main checkout from `git rev-parse --git-common-dir` (absolute)."""
    common = Path(common_dir)
    return common.parent if common.name == ".git" else common


def work_root(main):
    """`../astrolith-work/` next to the main checkout."""
    return Path(main).parent / WORK_DIR_NAME


def issue_folder(main, issue, slug):
    """The issue's worktree: `../astrolith-work/<N>-<slug>`."""
    name = slug if slug.startswith(f"{issue}-") else f"{issue}-{slug}"
    return work_root(main) / name


def check_folder(folder):
    """The validator worktree beside an issue's worktree: `<folder>-check`."""
    folder = Path(folder)
    return folder.with_name(folder.name + "-check")


def branch_folder(main, branch):
    """A worktree for a branch without an issue: its name, `/` as `-`."""
    return work_root(main) / branch.replace("/", "-")


def marker(comment_bodies):
    """The newest recorded choice: "yes", "no", or None (not asked yet)."""
    choice = None
    for body in comment_bodies:
        for found in MARKER.findall(body or ""):
            choice = found
    return choice


def marker_comment(mode):
    """The comment that records the owner's choice."""
    yes = mode == "worktree"
    text = ("new worktree (`../astrolith-work/`)" if yes
            else "normal (main checkout, as today)")
    return f"<!-- worktree:{'yes' if yes else 'no'} -->\nWay of working: **{text}**."


def free_bytes(path):
    """Free space on the drive holding `path` (its nearest existing parent)."""
    path = Path(path)
    while not path.exists() and path.parent != path:
        path = path.parent
    return shutil.disk_usage(path).free


def write_cargo_env(folder):
    """The worktree's cargo config: `cargo run` there reads its own settings."""
    config = Path(folder) / ".cargo" / "config.toml"
    config.parent.mkdir(parents=True, exist_ok=True)
    config.write_text(
        "# Written by development.py create-branch (#613): the game started\n"
        "# from this worktree uses its own settings file.\n"
        "[env]\n"
        f'ASTROLITH_SETTINGS = {{ value = "{SETTINGS_FILE}", relative = true }}\n',
        encoding="utf-8")
    (Path(folder) / ".agent-local").mkdir(exist_ok=True)
    return config


def claude_projects():
    """Claude Code's per-project folder root."""
    base = os.environ.get("CLAUDE_CONFIG_DIR") or str(Path.home() / ".claude")
    return Path(base) / "projects"


def project_key(path):
    """Claude Code's folder name for a working directory."""
    return re.sub(r"[^A-Za-z0-9]", "-", str(Path(path)))


def link_memory(main, folder):
    """Links the worktree's project memory to the main checkout's memory.

    Returns the link path, or None when the main memory folder is missing.
    """
    projects = claude_projects()
    target = projects / project_key(main) / "memory"
    if not target.exists():
        return None
    link = projects / project_key(folder) / "memory"
    link.parent.mkdir(parents=True, exist_ok=True)
    if link.exists() or link.is_symlink():
        return link
    if sys.platform.startswith("win"):
        subprocess.run(["cmd", "/c", "mklink", "/J", str(link), str(target)],
                       capture_output=True, text=True, check=False)
    else:
        os.symlink(target, link, target_is_directory=True)
    return link


def unlink_memory(folder):
    """Removes the worktree's memory link only, never the shared memory.

    The worktree agent's own session files beside it are left alone.
    """
    link = claude_projects() / project_key(folder) / "memory"
    if link.is_symlink():
        link.unlink()
    elif link.exists() and is_junction(link):
        # Removing a junction's directory entry leaves its target alone.
        os.rmdir(link)


def is_junction(path):
    """Whether `path` is a Windows junction (or any link), never a real folder."""
    checker = getattr(os.path, "isjunction", None)
    return bool(checker and checker(path)) or Path(path).is_symlink()


def evidence_root(main):
    """Where untracked evidence goes: the main checkout's `.agent/`."""
    return Path(main) / ".agent"
