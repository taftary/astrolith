#!/usr/bin/env python3
"""Hash and byte-compare files or directory trees, for the validator.

Two modes:

  hash    print a stable, sorted digest listing for a file or a directory tree
  compare take two paths, each a file or a directory, and compare them
          byte-for-byte, reporting the *first* difference and where it is

Why this exists: the validator's shell allowlist refuses `git hash-object` and
`git diff --no-index`, and `python scripts/validation/*` is already permitted, so
a script in this directory needs no sandbox change to be reachable. It is also
auditable in the repository like every other validation tool.

`compare` reports the first difference rather than a bare "they differ" because a
validator has to be able to *locate* a divergence; an unlocated difference is not
usable evidence.

Exit codes:
  0  identical
  1  a difference was found
  2  usage error, or a path could not be read

Determinism: directory listings are sorted, so the same tree always produces the
same output, on any machine. Comparison is on raw bytes, since the requirement is
byte-for-byte; no text normalisation is applied anywhere in this script.
"""

from __future__ import annotations

import argparse
import hashlib
import os
import sys
from pathlib import Path

CHUNK = 1 << 20
FMT = "%s  %s"


def die(msg: str) -> "int":
    """Report a usage or read error and return the usage exit code."""
    print("error: %s" % msg, file=sys.stderr)
    return 2


def digest(path: Path) -> str:
    """SHA-256 of a file's bytes, streamed so large files stay cheap."""
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for block in iter(lambda: fh.read(CHUNK), b""):
            h.update(block)
    return h.hexdigest()


def listing(root: Path) -> "list[tuple[str, Path]]":
    """Sorted (relative posix path, absolute path) for every file under root.

    Sorted so output is stable between runs and between machines. Directory
    symlinks are not followed, so a link cycle cannot make this loop.
    """
    found = []
    for dirpath, dirnames, filenames in os.walk(root, followlinks=False):
        dirnames.sort()
        for name in sorted(filenames):
            full = Path(dirpath) / name
            rel = full.relative_to(root).as_posix()
            found.append((rel, full))
    found.sort()
    return found


def first_byte_difference(left: Path, right: Path) -> "int | None":
    """Byte offset of the first differing byte between two files, or None."""
    with left.open("rb") as a, right.open("rb") as b:
        offset = 0
        while True:
            ca, cb = a.read(CHUNK), b.read(CHUNK)
            if not ca and not cb:
                return None
            if ca != cb:
                limit = min(len(ca), len(cb))
                for i in range(limit):
                    if ca[i] != cb[i]:
                        return offset + i
                # Equal up to the shorter read: one file ends here.
                return offset + limit
            offset += len(ca)


def byte_window(path: Path, offset: int, width: int = 16) -> "str":
    """A short printable window of bytes at offset, for the difference report."""
    with path.open("rb") as fh:
        fh.seek(offset)
        window = fh.read(width)
    return " ".join("%02x" % b for b in window) or "(end of file)"


def show_difference(what: str, offset: "int | None", left: "Path | None", right: "Path | None") -> None:
    """Print one difference line: what differs, where, and both values."""
    print("DIFFER at %s" % what)
    if offset is not None and left is not None and right is not None:
        print("  first differing byte: offset %d" % offset)
        print("  left  @%d : %s" % (offset, byte_window(left, offset)))
        print("  right @%d : %s" % (offset, byte_window(right, offset)))
        if left.stat().st_size != right.stat().st_size:
            print("  size : left %d bytes, right %d bytes"
                  % (left.stat().st_size, right.stat().st_size))


def cmd_hash(args: argparse.Namespace) -> int:
    """Print a sorted digest listing for a file or a directory tree."""
    target = Path(args.path)
    if not target.exists():
        return die("no such path: %s" % args.path)
    if target.is_file():
        try:
            print(FMT % (digest(target), target.name))
        except OSError as exc:
            return die("cannot read %s: %s" % (target, exc))
        return 0
    if not target.is_dir():
        return die("not a regular file or directory: %s" % args.path)
    try:
        entries = listing(target)
        digests = {rel: digest(full) for rel, full in entries}
    except OSError as exc:
        return die("cannot read tree %s: %s" % (target, exc))
    for rel in sorted(digests):
        print(FMT % (digests[rel], rel))
    print("# %d file(s) under %s" % (len(digests), target.as_posix()))
    return 0


def cmd_compare(args: argparse.Namespace) -> int:
    """Compare two files or two directory trees byte-for-byte."""
    left, right = Path(args.left), Path(args.right)
    for path in (left, right):
        if not path.exists():
            return die("no such path: %s" % path)
    if left.is_file() != right.is_file():
        return die("cannot compare a file with a directory: %s vs %s" % (left, right))

    # --- two files ---
    if left.is_file() and right.is_file():
        try:
            if digest(left) == digest(right):
                print("identical (1 file compared)")
                return 0
            offset = first_byte_difference(left, right)
        except OSError as exc:
            return die("cannot read: %s" % exc)
        show_difference("file %s" % left.name, offset, left, right)
        return 1

    # --- two directory trees: compare sorted relative-path sets, then digests ---
    try:
        l_entries = listing(left)
        r_entries = listing(right)
        l_map = {rel: full for rel, full in l_entries}
        r_map = {rel: full for rel, full in r_entries}
        l_digests = {rel: digest(full) for rel, full in l_entries}
        r_digests = {rel: digest(full) for rel, full in r_entries}
    except OSError as exc:
        return die("cannot read tree: %s" % exc)

    for rel in sorted(set(l_map) | set(r_map)):
        if rel not in r_map:
            print("DIFFER at %s: only in %s" % (rel, left.as_posix()))
            return 1
        if rel not in l_map:
            print("DIFFER at %s: only in %s" % (rel, right.as_posix()))
            return 1
        if l_digests[rel] != r_digests[rel]:
            offset = first_byte_difference(l_map[rel], r_map[rel])
            show_difference("file %s" % rel, offset, l_map[rel], r_map[rel])
            return 1

    count = len(l_map)
    print("identical (%d file(s) compared)" % count)
    return 0


def build_parser() -> argparse.ArgumentParser:
    """Build the argument parser, including the help text that documents both modes."""
    parser = argparse.ArgumentParser(
        prog="compare_files.py",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        description=__doc__.split("Why this exists")[0].strip(),
        epilog=(
            "modes:\n"
            "  hash PATH                 print a sorted digest listing for a file or a\n"
            "                            directory tree; one line per file as\n"
            "                            '<sha256>  <relative path>', stable between runs\n"
            "                            and between machines.\n"
            "  compare LEFT RIGHT        compare two paths, each a file or a directory,\n"
            "                            byte-for-byte. Prints 'identical' and exits 0 on a\n"
            "                            match. On a difference prints 'DIFFER at ...' naming\n"
            "                            the first differing file, and for a content\n"
            "                            difference the byte offset plus both byte windows,\n"
            "                            then exits 1. A file present in only one tree is a\n"
            "                            difference, never skipped.\n"
            "\n"
            "examples:\n"
            "  python scripts/validation/compare_files.py hash target\n"
            "  python scripts/validation/compare_files.py compare a/capture b/capture\n"
            "\n"
            "exit codes: 0 identical, 1 difference found, 2 usage or read error\n"
        ),
    )
    sub = parser.add_subparsers(dest="mode", required=True)

    p_hash = sub.add_parser("hash", help="print a sorted digest listing")
    p_hash.add_argument("path", help="file or directory to hash")
    p_hash.set_defaults(func=cmd_hash)

    p_cmp = sub.add_parser("compare", help="byte-compare two files or two directory trees")
    p_cmp.add_argument("left", help="first file or directory")
    p_cmp.add_argument("right", help="second file or directory")
    p_cmp.set_defaults(func=cmd_compare)

    return parser


def main(argv: "list[str] | None" = None) -> int:
    args = build_parser().parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())