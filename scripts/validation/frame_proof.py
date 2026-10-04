"""Frame proof for visual changes (issue #134, T1 core + T2 capture).

Stdlib only. Two modes sharing the compare/fragment core:

Offline (T1): two given capture dirs, no network.
  --before-dir A --after-dir B --out-dir O --issue N
  --before-sha <sha> --after-sha <sha> --visual yes|no --dry-run

Captured (T2): capture after at HEAD and before at the base worktree.
  --issue N --sha <sha> --visual yes|no [--base <ref>] [--out-dir O]
  --dry-run
  HEAD must equal --sha (else exit 2). Base defaults to
  ``git merge-base HEAD origin/main`` (``HEAD~1`` when HEAD is already
  on ``origin/main``). The base is captured in a detached worktree that
  is always removed, then the offline core compares and writes
  ``frame-proof.md`` + ``frame-proof.json`` into
  ``.agent/validation/issue-N/<ts>/`` (or --out-dir).

Exit contract (stdout machine lines, no reserved vocabulary words):
  0  FRAME-PROOF-OK changed=<k> published=<m>
  1  FRAME-PROOF-MISMATCH levels=[...] <what was claimed vs seen>
  2  FRAME-PROOF-BLOCKED <missing piece>
"""

from __future__ import annotations

import argparse
import binascii
import datetime
import os
import struct
import subprocess
import sys
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

SCALE = 2
EXPECTED_COUNT = 11

PNG_SIG = b"\x89PNG\r\n\x1a\n"


def parse_ppm(path: Path) -> tuple[int, int, bytes]:
    """Parse a P3 text PPM file, return (width, height, raw RGB bytes).

    Raises ValueError or OSError when the file cannot be used.
    """
    raw = path.read_bytes()
    tokens: list[bytes] = []
    magic: bytes | None = None
    pos = 0
    n = len(raw)

    def skip_ws_comments(p: int) -> int:
        while p < n:
            c = raw[p]
            if c in (0x20, 0x09, 0x0A, 0x0D):
                p += 1
            elif c == 0x23:  # '#': comment to end of line
                while p < n and raw[p] not in (0x0A, 0x0D):
                    p += 1
            else:
                break
        return p

    pos = skip_ws_comments(pos)
    start = pos
    while pos < n and raw[pos] not in (0x20, 0x09, 0x0A, 0x0D):
        pos += 1
    magic = raw[start:pos]
    if magic != b"P3":
        raise ValueError(f"not P3 PPM: {path.name}")
    pos = skip_ws_comments(pos)
    # width, height, maxval, then pixels: tokenise the rest
    rest = bytearray()
    # Re-scan the remainder splitting on whitespace, skipping comments.
    buf: list[bytes] = []
    p = pos
    cur = bytearray()
    while p < n:
        c = raw[p]
        if c == 0x23:
            if cur:
                buf.append(bytes(cur))
                cur = bytearray()
            while p < n and raw[p] not in (0x0A, 0x0D):
                p += 1
        elif c in (0x20, 0x09, 0x0A, 0x0D):
            if cur:
                buf.append(bytes(cur))
                cur = bytearray()
            p += 1
        else:
            cur.append(c)
            p += 1
    if cur:
        buf.append(bytes(cur))
    tokens = buf
    _ = rest
    if len(tokens) < 3:
        raise ValueError(f"short PPM header: {path.name}")
    try:
        width = int(tokens[0])
        height = int(tokens[1])
        maxval = int(tokens[2])
    except ValueError:
        raise ValueError(f"bad PPM header: {path.name}")
    if width <= 0 or height <= 0:
        raise ValueError(f"bad PPM size: {path.name}")
    if maxval != 255:
        raise ValueError(f"bad PPM maxval: {path.name}")
    vals = tokens[3:]
    want = width * height * 3
    if len(vals) != want:
        raise ValueError(
            f"PPM pixel count {len(vals)} != {want}: {path.name}"
        )
    rgb = bytearray(want)
    for i, tok in enumerate(vals):
        try:
            v = int(tok)
        except ValueError:
            raise ValueError(f"bad PPM pixel: {path.name}")
        if v < 0 or v > 255:
            raise ValueError(f"PPM pixel out of range: {path.name}")
        rgb[i] = v
    return width, height, bytes(rgb)


def scale_2x(width: int, height: int, rgb: bytes) -> tuple[int, int, bytes]:
    """Nearest-neighbour 2x scale of packed RGB bytes."""
    w2, h2 = width * SCALE, height * SCALE
    out = bytearray(w2 * h2 * 3)
    for y in range(height):
        for x in range(width):
            si = (y * width + x) * 3
            r, g, b = rgb[si], rgb[si + 1], rgb[si + 2]
            for dy in range(SCALE):
                row = (y * SCALE + dy) * w2 + x * SCALE
                for dx in range(SCALE):
                    di = (row + dx) * 3
                    out[di] = r
                    out[di + 1] = g
                    out[di + 2] = b
    return w2, h2, bytes(out)


def _chunk(ctype: bytes, data: bytes) -> bytes:
    out = struct.pack(">I", len(data)) + ctype + data
    out += struct.pack(">I", binascii.crc32(ctype + data) & 0xFFFFFFFF)
    return out


def encode_png(width: int, height: int, rgb: bytes) -> bytes:
    """Encode packed RGB bytes as a truecolor PNG (8-bit, color type 2)."""
    if len(rgb) != width * height * 3:
        raise ValueError("RGB length does not match width/height")
    ihdr = struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)
    raw = bytearray()
    stride = width * 3
    for y in range(height):
        raw.append(0)
        raw += rgb[y * stride:(y + 1) * stride]
    compressed = zlib.compress(bytes(raw), 6)
    return (
        PNG_SIG
        + _chunk(b"IHDR", ihdr)
        + _chunk(b"IDAT", compressed)
        + _chunk(b"IEND", b"")
    )


def ppm_to_png_2x(ppm_path: Path) -> tuple[int, int, bytes]:
    """Parse one PPM file and return (width, height, PNG bytes) at 2x."""
    w, h, rgb = parse_ppm(ppm_path)
    w2, h2, big = scale_2x(w, h, rgb)
    return w2, h2, encode_png(w2, h2, big)


def level_key(frame_name: str) -> str:
    """Level key from a frame filename, e.g. frame-L1-X.ppm -> L1-X."""
    base = frame_name
    if base.startswith("frame-"):
        base = base[len("frame-"):]
    if base.endswith(".ppm"):
        base = base[: -len(".ppm")]
    return base


def level_sort_key(key: str) -> tuple[int, str]:
    """Sort by the numeric level first (L1..L11), then by full key."""
    num = 10**9
    rest = key
    if key.startswith("L"):
        digits = ""
        for ch in key[1:]:
            if ch.isdigit():
                digits += ch
            else:
                break
        if digits:
            num = int(digits)
            rest = key
    return (num, rest)


def frame_files(capture_dir: Path) -> dict[str, Path]:
    """Map level key -> frame PPM path for one capture dir, sorted by level."""
    found: dict[str, Path] = {}
    for p in sorted(capture_dir.glob("frame-*.ppm")):
        if p.is_file():
            found[level_key(p.name)] = p
    return dict(sorted(found.items(), key=lambda kv: level_sort_key(kv[0])))


def compare_dirs(before_dir: Path, after_dir: Path) -> tuple[list[str], list[str]]:
    """Byte-compare frame PPM pairs.

    Returns (changed, all_levels). Raises OSError/ValueError when a
    directory or a paired file cannot be read.
    """
    if not before_dir.is_dir():
        raise OSError(f"missing before dir: {before_dir}")
    if not after_dir.is_dir():
        raise OSError(f"missing after dir: {after_dir}")
    before = frame_files(before_dir)
    after = frame_files(after_dir)
    if not before and not after:
        raise OSError("no frame files in either dir")
    if set(before) != set(after):
        missing = sorted(set(before) ^ set(after))
        raise OSError(f"frame set differs: {','.join(missing)}")
    changed: list[str] = []
    for key in before:
        if before[key].read_bytes() != after[key].read_bytes():
            changed.append(key)
    changed.sort(key=level_sort_key)
    all_levels = sorted(before, key=level_sort_key)
    return changed, all_levels


def short_sha(sha: str) -> str:
    return sha[:7] if len(sha) >= 7 else sha


def build_fragment(
    changed: list[str],
    all_levels: list[str],
    before_sha: str,
    after_sha: str,
    url_for: "callable[[str, str], str]",
) -> str:
    """Build the validator verdict fragment. Starts with a blank line.

    url_for(kind, level) -> URL where kind is 'before' or 'after'.
    """
    b7 = short_sha(before_sha)
    a7 = short_sha(after_sha)
    lines: list[str] = [""]
    if not changed:
        lines.append(
            f"Visual: no change — {len(all_levels)} frames identical to main (base {b7})"
        )
        return "\n".join(lines) + "\n"
    lines.append(
        f"Visual: changed {len(changed)}/{len(all_levels)} frames vs main (base {b7}, after {a7})"
    )
    lines.append("")
    lines.append("| level | before | after |")
    lines.append("|---|---|---|")
    for level in changed:
        b_url = url_for("before", level)
        a_url = url_for("after", level)
        lines.append(
            f"| {level} | ![before {level} @{b7}]({b_url}) "
            f"| ![after {level} @{a7}]({a_url}) |"
        )
    lines.append("")
    lines.append(f"<sub>Before {before_sha} / after {after_sha}</sub>")
    lines.append("")
    lines.append("<details>")
    lines.append(f"<summary>All {len(all_levels)} after frames (after {a7})</summary>")
    lines.append("")
    for level in all_levels:
        a_url = url_for("after", level)
        lines.append(f"![after {level} @{a7}]({a_url})")
        lines.append("")
    lines.append("</details>")
    return "\n".join(lines) + "\n"


def placeholder_url(issue: int, sha: str, kind: str, level: str) -> str:
    return (
        f"https://placeholder.invalid/issue-{issue}-{short_sha(sha)}-"
        f"{kind}-{level}.png"
    )


def run_offline(
    before_dir: Path,
    after_dir: Path,
    out_dir: Path,
    issue: int,
    before_sha: str,
    after_sha: str,
    visual: str,
    dry_run: bool,
) -> int:
    """Offline compare + fragment write. Returns the process exit code."""
    try:
        changed, all_levels = compare_dirs(before_dir, after_dir)
    except (OSError, ValueError) as exc:
        print(f"FRAME-PROOF-BLOCKED {exc}")
        return 2

    # Convert frames to PNG so the PNG path is proven on real captures.
    # Convert failures name the level and block the verdict (AC8).
    png_out: dict[str, dict[str, str]] = {}
    try:
        out_dir.mkdir(parents=True, exist_ok=True)
        before_map = frame_files(before_dir)
        after_map = frame_files(after_dir)
        for level in all_levels:
            _, _, b_png = ppm_to_png_2x(before_map[level])
            _, _, a_png = ppm_to_png_2x(after_map[level])
            (out_dir / f"before-{level}.png").write_bytes(b_png)
            (out_dir / f"after-{level}.png").write_bytes(a_png)
            if dry_run:
                png_out[level] = {
                    "before": placeholder_url(issue, before_sha, "before", level),
                    "after": placeholder_url(issue, after_sha, "after", level),
                }
            else:
                png_out[level] = {
                    "before": str(out_dir / f"before-{level}.png"),
                    "after": str(out_dir / f"after-{level}.png"),
                }
    except (OSError, ValueError) as exc:
        print(f"FRAME-PROOF-BLOCKED convert {exc}")
        return 2

    def url_for(kind: str, level: str) -> str:
        return png_out[level][kind]

    fragment = build_fragment(changed, all_levels, before_sha, after_sha, url_for)
    try:
        (out_dir / "frame-proof.md").write_text(fragment, encoding="utf-8")
        import json as _json

        doc = {
            "issue": issue,
            "before_sha": before_sha,
            "after_sha": after_sha,
            "visual": visual,
            "changed": changed,
            "all_levels": all_levels,
            "dry_run": dry_run,
            "urls": png_out,
        }
        (out_dir / "frame-proof.json").write_text(
            _json.dumps(doc, indent=2) + "\n", encoding="utf-8"
        )
    except OSError as exc:
        print(f"FRAME-PROOF-BLOCKED write {exc}")
        return 2

    published = 0  # T3 adds the upload step; offline runs publish nothing.
    if visual == "yes" and not changed:
        print(
            f"FRAME-PROOF-MISMATCH levels=[] "
            f"claimed visual but {len(all_levels)} frames match base {short_sha(before_sha)}"
        )
        return 1
    if visual == "no" and changed:
        print(
            f"FRAME-PROOF-MISMATCH levels=[{','.join(changed)}] "
            f"claimed no visual change but {len(changed)} frame(s) differ"
        )
        return 1
    print(f"FRAME-PROOF-OK changed={len(changed)} published={published}")
    return 0


def _git(args: list[str], cwd: Path, timeout: int = 60) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git"] + args, capture_output=True, text=True, encoding="utf-8",
        errors="replace", cwd=str(cwd), timeout=timeout)


def git_rev_parse(ref: str, cwd: Path = ROOT) -> str:
    """Full SHA for a git ref. Raises OSError when it cannot be resolved."""
    try:
        p = _git(["rev-parse", ref], cwd)
    except (OSError, subprocess.SubprocessError) as exc:
        raise OSError(f"cannot run git rev-parse {ref}: {exc}")
    if p.returncode != 0:
        tail = (p.stderr or "").strip().splitlines()
        hint = tail[-1][:160] if tail else f"rc={p.returncode}"
        raise OSError(f"cannot resolve {ref}: {hint}")
    sha = (p.stdout or "").strip().split()
    if not sha or len(sha[0]) != 40:
        raise OSError(f"cannot resolve {ref}: no SHA in output")
    return sha[0]


def resolve_base_sha(base_arg: "str | None", cwd: Path = ROOT) -> tuple[str, str]:
    """Return (base description, base SHA).

    --base wins when given. Otherwise the merge-base of HEAD with
    origin/main; when HEAD is already on origin/main (post-merge run)
    the parent HEAD~1 is the base instead.
    """
    if base_arg:
        return base_arg, git_rev_parse(base_arg, cwd)
    head = git_rev_parse("HEAD", cwd)
    origin = git_rev_parse("origin/main", cwd)
    if head == origin:
        return "HEAD~1", git_rev_parse("HEAD~1", cwd)
    p = _git(["merge-base", "HEAD", "origin/main"], cwd)
    if p.returncode != 0:
        tail = (p.stderr or "").strip().splitlines()
        hint = tail[-1][:160] if tail else f"rc={p.returncode}"
        raise OSError(f"cannot resolve merge-base HEAD origin/main: {hint}")
    sha = (p.stdout or "").strip().split()
    if not sha or len(sha[0]) != 40:
        raise OSError("cannot resolve merge-base HEAD origin/main: no SHA in output")
    return "merge-base HEAD origin/main", sha[0]


def run_capture(cwd: Path, dest_dir: Path, label: str, timeout: int = 900
                ) -> tuple[bool, str, int, str, str]:
    """Run the headless capture in cwd into dest_dir.

    Returns (ok, piece, rc, stdout, stderr). The same target dir serves
    both checkouts; cargo fingerprints keep each build correct.
    """
    dest_dir.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ)
    env["CARGO_TARGET_DIR"] = str(ROOT / "target")
    try:
        p = subprocess.run(
            ["cargo", "run", "--locked", "-p", "universe-app",
             "--", "--capture", str(dest_dir)],
            capture_output=True, text=True, encoding="utf-8",
            errors="replace", cwd=str(cwd), timeout=timeout, env=env)
    except (OSError, subprocess.SubprocessError) as exc:
        return False, f"capture {label} did not start: {exc}", 127, "", ""
    except Exception as exc:  # timeout surfaces here on some platforms
        return False, f"capture {label} did not complete: {exc}", 124, "", ""
    out, err = p.stdout or "", p.stderr or ""
    try:
        frames = [f for f in dest_dir.glob("frame-*.ppm") if f.is_file()]
    except OSError as exc:
        return False, f"capture {label} unreadable dir: {exc}", p.returncode, out, err
    ok = p.returncode == 0 and "CAPTURE-OK" in out and len(frames) > 0
    piece = (f"capture {label} rc={p.returncode} "
             f"CAPTURE-OK={'CAPTURE-OK' in out} frames={len(frames)}")
    return ok, piece, p.returncode, out, err


def utc_stamp() -> str:
    return datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%SZ")


def run_captured(issue: int, sha: str, visual: str, base_arg: "str | None",
                 out_dir_arg: "Path | None", dry_run: bool) -> int:
    """Captured mode: after at HEAD, before at the base worktree."""
    try:
        head = git_rev_parse("HEAD", ROOT)
    except OSError as exc:
        print(f"FRAME-PROOF-BLOCKED {exc}")
        return 2
    if head != sha:
        print(f"FRAME-PROOF-BLOCKED HEAD {head} differs from --sha {sha}")
        return 2
    try:
        base_desc, base_sha = resolve_base_sha(base_arg, ROOT)
    except OSError as exc:
        print(f"FRAME-PROOF-BLOCKED {exc}")
        return 2

    out_dir = out_dir_arg or (ROOT / ".agent" / "validation"
                              / f"issue-{issue}" / utc_stamp())
    after_dir = out_dir / "after"
    before_dir = out_dir / "before"
    wt_dir = out_dir / "base-wt"
    try:
        out_dir.mkdir(parents=True, exist_ok=True)
    except OSError as exc:
        print(f"FRAME-PROOF-BLOCKED cannot create {out_dir}: {exc}")
        return 2

    ok, piece, rc, out, err = run_capture(ROOT, after_dir, "after")
    (out_dir / "capture-after.log").write_text(
        f"$ cargo run --locked -p universe-app -- --capture {after_dir}\n"
        f"rc={rc}\n---stdout---\n{out}\n---stderr---\n{err}",
        encoding="utf-8")
    if not ok:
        print(f"FRAME-PROOF-BLOCKED {piece}")
        return 2

    p = _git(["worktree", "add", "--detach", str(wt_dir), base_sha], ROOT, timeout=120)
    if p.returncode != 0:
        tail = ((p.stderr or "") + "\n" + (p.stdout or "")).strip().splitlines()
        hint = tail[-1][:200] if tail else f"rc={p.returncode}"
        print(f"FRAME-PROOF-BLOCKED worktree add {base_sha[:7]}: {hint}")
        return 2
    try:
        ok, piece, rc, out, err = run_capture(wt_dir, before_dir, "before")
        (out_dir / "capture-before.log").write_text(
            f"$ cargo run --locked -p universe-app -- --capture {before_dir}\n"
            f"cwd={wt_dir} base={base_desc} {base_sha}\n"
            f"rc={rc}\n---stdout---\n{out}\n---stderr---\n{err}",
            encoding="utf-8")
        if not ok:
            print(f"FRAME-PROOF-BLOCKED {piece}")
            return 2
    finally:
        q = _git(["worktree", "remove", "--force", str(wt_dir)], ROOT, timeout=120)
        if q.returncode != 0:
            tail = ((q.stderr or "") + "\n" + (q.stdout or "")).strip().splitlines()
            hint = tail[-1][:200] if tail else f"rc={q.returncode}"
            print(f"FRAME-PROOF-BLOCKED worktree remove {wt_dir}: {hint}")
            return 2

    return run_offline(before_dir, after_dir, out_dir, issue,
                       base_sha, sha, visual, dry_run)


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog="frame_proof.py",
        description="Compare capture frames and build the Issue proof fragment (offline dirs or worktree capture).",
    )
    p.add_argument("--before-dir", default=None)
    p.add_argument("--after-dir", default=None)
    p.add_argument("--out-dir", default=None)
    p.add_argument("--issue", default=None, type=int)
    p.add_argument("--before-sha", default=None)
    p.add_argument("--after-sha", default=None)
    p.add_argument("--sha", default=None,
                   help="captured mode: validated commit; HEAD must equal it")
    p.add_argument("--base", default=None,
                   help="captured mode: base ref override (default: merge-base with origin/main)")
    p.add_argument("--visual", default=None, choices=["yes", "no"])
    p.add_argument("--dry-run", action="store_true")
    return p


def _need(args: argparse.Namespace, *names: str) -> "str | None":
    for name in names:
        if getattr(args, name) is None:
            return name
    return None


def main(argv: "list[str] | None" = None) -> int:
    args = build_parser().parse_args(argv)
    try:
        if args.sha is not None:
            missing = _need(args, "issue", "visual")
            if missing:
                print(f"FRAME-PROOF-BLOCKED missing --{missing}")
                return 2
            if args.before_dir is not None or args.after_dir is not None:
                print("FRAME-PROOF-BLOCKED --sha runs its own captures")
                return 2
            return run_captured(
                args.issue, args.sha, args.visual,
                args.base,
                Path(args.out_dir) if args.out_dir else None,
                args.dry_run,
            )
        missing = _need(args, "before_dir", "after_dir", "out_dir",
                         "issue", "before_sha", "after_sha", "visual")
        if missing:
            print(f"FRAME-PROOF-BLOCKED missing --{missing.replace('_', '-')}")
            return 2
        return run_offline(
            Path(str(args.before_dir)),
            Path(str(args.after_dir)),
            Path(str(args.out_dir)),
            args.issue,
            str(args.before_sha),
            str(args.after_sha),
            str(args.visual),
            args.dry_run,
        )
    except (OSError, ValueError) as exc:
        print(f"FRAME-PROOF-BLOCKED {exc}")
        return 2


if __name__ == "__main__":
    sys.exit(main())
