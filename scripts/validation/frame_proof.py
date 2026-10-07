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
import hashlib
import os
import struct
import subprocess
import sys
import time
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
            f"Visual: none ({len(all_levels)} frames identical to main; "
            f"base {b7}, after {a7})"
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


RELEASE_TAG = "validation-evidence"
RELEASE_NAME = "Validation evidence (do not use)"


def asset_name(issue: int, sha: str, kind: str, level: str) -> str:
    """Release asset name for one proof PNG (AC5)."""
    return f"issue-{issue}-{short_sha(sha)}-{kind}-{level}.png"


def _gh(args: list[str], timeout: int = 120) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["gh"] + args, capture_output=True, text=True, encoding="utf-8",
        errors="replace", cwd=str(ROOT), timeout=timeout)


def repo_slug() -> str:
    """owner/repo for the checkout. Raises OSError when it cannot be read."""
    try:
        p = _gh(["repo", "view", "--json", "nameWithOwner", "--jq", ".nameWithOwner"])
    except (OSError, subprocess.SubprocessError) as exc:
        raise OSError(f"cannot read repo slug: {exc}")
    if p.returncode != 0:
        raise OSError("cannot read repo slug: gh repo view did not complete")
    slug = (p.stdout or "").strip()
    if "/" not in slug:
        raise OSError("cannot read repo slug: no owner/repo in output")
    return slug


def _api_json(args: list[str], timeout: int = 120) -> "tuple[int, object]":
    """Run gh api, return (returncode, parsed JSON or None). Never echoes bodies."""
    try:
        p = _gh(["api"] + args, timeout)
    except (OSError, subprocess.SubprocessError) as exc:
        raise OSError(f"cannot run gh api: {exc}")
    if p.returncode != 0:
        return p.returncode, None
    import json as _json

    try:
        return 0, _json.loads(p.stdout or "null")
    except ValueError:
        raise OSError("gh api returned a body that is not JSON")


def ensure_evidence_release(slug: str) -> int:
    """Return the release id for RELEASE_TAG, creating and publishing as needed.

    The entry stays a published pre-release, so it is downloadable but is
    never marked Latest. Raises OSError naming the piece that is missing.
    """
    rc, data = _api_json(["repos/" + slug + "/releases/tags/" + RELEASE_TAG])
    release_id: int | None = None
    if rc == 0 and isinstance(data, dict) and data.get("id"):
        try:
            release_id = int(data["id"])
        except (TypeError, ValueError):
            raise OSError(f"release {RELEASE_TAG} id is not a number")
        if data.get("draft") or not data.get("prerelease"):
            rc2, _ = _api_json(["repos/" + slug + f"/releases/{release_id}",
                                "-X", "PATCH", "-F", "draft=false",
                                "-F", "prerelease=true"])
            if rc2 != 0:
                raise OSError(f"release {RELEASE_TAG} could not be published")
        return release_id
    rc, data = _api_json(["repos/" + slug + "/releases", "-X", "POST",
                          "-f", "tag_name=" + RELEASE_TAG,
                          "-f", "name=" + RELEASE_NAME,
                          "-F", "prerelease=true", "-F", "draft=false"])
    if rc != 0 or not isinstance(data, dict) or not data.get("id"):
        raise OSError(f"release {RELEASE_TAG} could not be created")
    try:
        return int(data["id"])
    except (TypeError, ValueError):
        raise OSError(f"release {RELEASE_TAG} id is not a number")


def _fetch_200(url: str, timeout: int = 60) -> bytes:
    """GET a URL, demanding HTTP 200 and a non-empty body. Stdlib only."""
    import urllib.request

    req = urllib.request.Request(url, method="GET",
                                 headers={"User-Agent": "frame-proof"})
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            status = getattr(resp, "status", 200)
            body = resp.read()
    except Exception as exc:
        raise OSError(f"fetch {url[:80]} did not complete")
    if status != 200:
        raise OSError(f"fetch {url[:80]} gave HTTP {status}")
    if not body:
        raise OSError(f"fetch {url[:80]} gave an empty body")
    return body


def publish_pngs(issue: int, sha: str,
                 png_paths: "dict[tuple[str, str], Path]",
                 trail: list[str]
                 ) -> dict[str, dict[str, str]]:
    """Upload proof PNG files to the evidence release with overwrite (AC5).

    Returns {level: {"before": url, "after": url}} with the post-publish
    browser_download_url of each asset, appending one audit line per
    asset to trail (written to publish.log by the caller on success
    and on failure). Every URL is fetched back with HTTP 200 before
    return. Raises OSError("publish <detail>") otherwise; nothing
    sensitive is ever printed (auth stays inside gh, bodies stay in
    the trail).
    """
    slug = repo_slug()
    release_id = ensure_evidence_release(slug)
    trail.append(f"release {RELEASE_TAG} id={release_id}")
    rc, data = _api_json(["repos/" + slug + f"/releases/{release_id}/assets",
                          "-X", "GET", "-F", "per_page=100", "--paginate"])
    existing: dict[str, int] = {}
    if rc == 0 and isinstance(data, list):
        for asset in data:
            if isinstance(asset, dict) and asset.get("name") and asset.get("id"):
                try:
                    existing[str(asset["name"])] = int(asset["id"])
                except (TypeError, ValueError):
                    continue
    urls: dict[str, dict[str, str]] = {}
    if rc != 0:
        trail.append(f"list assets rc={rc}: continuing without overwrite")
    else:
        trail.append(f"list assets ok: {len(existing)} present")
    # prune-superseded: this issue keeps only its newest commit's assets.
    # Older-commit assets (same issue prefix, different short SHA) are
    # removed first so the release never fills up with superseded rounds.
    current = short_sha(sha)
    prefix = f"issue-{issue}-"
    for old_name in sorted(existing):
        if not old_name.startswith(prefix):
            continue
        tag = old_name[len(prefix):].split("-", 1)[0]
        if tag == current:
            continue
        rc, _ = _api_json(
            ["repos/" + slug + f"/releases/assets/{existing[old_name]}", "-X", "DELETE"])
        if rc != 0:
            raise OSError(f"publish {old_name}: superseded asset could not be removed")
        trail.append(f"prune-superseded {old_name}")
    for kind, level in sorted(png_paths):
        name = asset_name(issue, sha, kind, level)
        if name in existing:
            rc, _ = _api_json(
                ["repos/" + slug + f"/releases/assets/{existing[name]}", "-X", "DELETE"])
            if rc != 0:
                raise OSError(f"publish {name}: existing asset could not be removed")
            trail.append(f"deleted {name}")
        src = png_paths[(kind, level)]
        upload = (f"https://uploads.github.com/repos/{slug}/releases/"
                  f"{release_id}/assets?name={name}")
        # retry-with-backoff: transient upload misses are retried a few
        # times with waits before the run gives up on the asset.
        attempts = 4
        waits = (2, 5, 10)
        p = None
        body = ""
        for attempt in range(1, attempts + 1):
            try:
                p = _gh([  # noqa: E501 - uploads host form proven in preflight; --hostname does not work
                    "api", upload, "-X", "POST",
                    "-H", "Content-Type: image/png", "--input", str(src)], timeout=300)
            except (OSError, subprocess.SubprocessError) as exc:
                if attempt == attempts:
                    raise OSError(f"publish {name}: upload did not start")
                time.sleep(waits[attempt - 1] if attempt - 1 < len(waits) else waits[-1])
                continue
            if p.returncode == 0:
                break
            body = ((p.stderr or "") + "\n" + (p.stdout or "")).strip()
            # full-errors-logged: the complete upload error body stays in
            # the trail (publish.log) instead of a 160-char hint, so the
            # GitHub errors[] array with the real reason is never lost.
            trail.append(f"upload {name} attempt {attempt} rc={p.returncode}: {body[:4000]}")
            if "already_exists" in body:
                raise OSError(f"publish {name}: already exists, overwrite unavailable")
            if attempt == attempts:
                raise OSError(f"publish {name}: upload did not complete")
            time.sleep(waits[attempt - 1] if attempt - 1 < len(waits) else waits[-1])
        if p is None or p.returncode != 0:
            raise OSError(f"publish {name}: upload did not complete")
        import json as _json

        try:
            got = _json.loads(p.stdout or "null")
        except ValueError:
            raise OSError(f"publish {name}: upload reply is not JSON")
        url = got.get("browser_download_url", "") if isinstance(got, dict) else ""
        if not url or not isinstance(url, str):
            raise OSError(f"publish {name}: no download URL in reply")
        size = len(_fetch_200(url))
        trail.append(f"uploaded {name} verified={size}B")
        urls.setdefault(level, {})[kind] = url
    return urls


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
    png_paths: dict[tuple[str, str], Path] = {}
    try:
        out_dir.mkdir(parents=True, exist_ok=True)
        before_map = frame_files(before_dir)
        after_map = frame_files(after_dir)
        for level in all_levels:
            _, _, b_png = ppm_to_png_2x(before_map[level])
            _, _, a_png = ppm_to_png_2x(after_map[level])
            b_path = out_dir / f"before-{level}.png"
            a_path = out_dir / f"after-{level}.png"
            b_path.write_bytes(b_png)
            a_path.write_bytes(a_png)
            png_paths[("before", level)] = b_path
            png_paths[("after", level)] = a_path
    except (OSError, ValueError) as exc:
        print(f"FRAME-PROOF-BLOCKED convert {exc}")
        return 2

    png_out: dict[str, dict[str, str]] = {}
    published = 0
    if dry_run:
        for level in all_levels:
            png_out[level] = {
                "before": placeholder_url(issue, before_sha, "before", level),
                "after": placeholder_url(issue, after_sha, "after", level),
            }
    else:
        trail: list[str] = []
        try:
            png_out = publish_pngs(issue, after_sha, png_paths, trail)
        except OSError as exc:
            try:
                (out_dir / "publish.log").write_text(
                    "\n".join(trail) + f"\nBLOCKED {exc}\n", encoding="utf-8")
            except OSError:
                pass
            print(f"FRAME-PROOF-BLOCKED {exc}")
            return 2
        try:
            (out_dir / "publish.log").write_text(
                "\n".join(trail) + "\n", encoding="utf-8")
        except OSError as exc:
            print(f"FRAME-PROOF-BLOCKED write {exc}")
            return 2
        published = sum(len(v) for v in png_out.values())

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
            "release": None if dry_run else RELEASE_TAG,
            "urls": png_out,
        }
        (out_dir / "frame-proof.json").write_text(
            _json.dumps(doc, indent=2) + "\n", encoding="utf-8"
        )
    except OSError as exc:
        print(f"FRAME-PROOF-BLOCKED write {exc}")
        return 2

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


def run_capture(cwd: Path, dest_dir: Path, label: str, target_dir: "Path | None" = None,
                timeout: int = 2400
                ) -> tuple[bool, str, int, str, str]:
    """Run the headless capture in cwd into dest_dir.

    Returns (ok, piece, rc, stdout, stderr). Captures build into
    target_dir: the main workspace target dir is left alone because a
    running app locks its exe on Windows and a shared dir would fail
    the base rebuild. Callers pass a stable cache (see
    shared_target_dir); cargo fingerprints keep each build correct.
    """
    dest_dir.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ)
    if target_dir is not None:
        env["CARGO_TARGET_DIR"] = str(target_dir)
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


def shared_target_dir(role: str) -> Path:
    """Stable isolated cargo target dir for proof captures, one per role.

    The main workspace target dir is left alone: a running app locks
    its exe on Windows, which fails any rebuild sharing that dir, and
    a fresh dir per run would rebuild Bevy from scratch every time.
    After and before builds must not share one dir either: both
    checkouts produce the same exe path, so one build clobbers the
    other binary while its unit still looks fresh. Each role keeps its
    own cache with stable source paths, so normal fingerprinting
    applies within it. If a link fails with LNK1140 (program database
    limit), delete that role dir and let it rebuild fresh; never seed
    one role dir by copying the other.

    These caches live under `.agent/cache/` in the repository, not in
    the OS temp directory. Temp cleanup used to delete them silently,
    and the next proof then paid a full cold rebuild on both sides:
    measured 26 min 55 s and 25 min 45 s (189 and 291 crates compiled)
    on issue 157, plus roughly 10 GB of writes, which is what exhausted
    the dev PC's disk and caused the LNK1140 link failures. `.agent/` is
    already the home for machine-local gitignored state and no script
    walks the repo tree, so an ignored cache dir there is inert.
    """
    return ROOT / ".agent" / "cache" / f"frame-proof-target-{role}"


def sha256_file(path: Path) -> str:
    """SHA-256 of a file's bytes, streamed so a large exe stays cheap."""
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for block in iter(lambda: fh.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def app_exe(target_dir: Path) -> "Path | None":
    """The built universe-app executable inside one role's target dir."""
    debug = target_dir / "debug"
    for name in ("universe-app.exe", "universe-app"):
        candidate = debug / name
        if candidate.is_file():
            return candidate
    return None


def build_identity(cwd: Path, target_dir: Path) -> str:
    """One evidence line naming exactly what produced a capture.

    Replaces the post-capture `cargo clean -p universe-app -p universe-render
    -p universe-core`, which existed only so a stale binary could not
    survive a run and cost a recompile of all three workspace crates on
    every proof (20-60 s warm, 26 min cold). Cargo is already
    authoritative here: it fingerprints the checkout by content, and
    `git worktree add` rewrites source mtimes to the checkout time, so no
    mtime comparison can tell "cached and correct" from "stale". This
    line records the source commit, whether that tree was clean, and the
    executable's own digest, so a reader can see which binary produced
    the frames instead of inferring it. A stale binary still cannot
    produce a false pass: the proof compares before against after, so a
    stale side collapses them and a claimed visual change fails.

    Evidence gathering never fails the proof: every error is reported
    inside the line (`head=(unreadable: ...)`, `sha256=(unreadable...)`),
    never raised, because a diagnostic must not be able to turn a good
    capture into a BLOCKED one.
    """
    try:
        head = git_rev_parse("HEAD", cwd)
    except OSError as exc:
        return f"identity head=(unreadable: {exc}) dirty=? exe=? sha256=?"
    status = _git(["status", "--porcelain", "--untracked-files=no"],
                  cwd, timeout=60)
    dirty = bool((status.stdout or "").strip()) if status.returncode == 0 else True
    exe = app_exe(target_dir)
    if exe is None:
        return f"identity head={head} dirty={dirty} exe=(none)"
    try:
        digest = sha256_file(exe)
    except OSError as exc:
        return f"identity head={head} dirty={dirty} exe={exe.name} sha256=(unreadable: {exc})"
    return f"identity head={head} dirty={dirty} exe={exe.name} sha256={digest}"


def base_worktree_dir() -> Path:
    """Stable detached-worktree path for base captures (outside the repo).

    One fixed path keeps build object paths stable across runs, so the
    isolated target cache stays bounded. It is removed before each add
    (plus `git worktree prune` for stale registrations) and always
    removed afterwards, so `git worktree list` shows only the main tree.
    """
    import tempfile

    return Path(tempfile.gettempdir()) / "astrolith-frame-proof-base-wt"


def utc_stamp() -> str:
    return datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%SZ")


def capturable_changed_files(base_sha: str, head_sha: str) -> "list[str]":
    """Source files base..head that can alter a captured byte (AC4).

    Only product code can move a captured pixel: `crates/` sources plus
    the workspace manifest and lockfile. Docs, skills, workflow text,
    and repo tooling cannot, so they never force a fresh capture.
    Raises OSError when the diff cannot be read.
    """
    p = _git(["diff", "--name-only", f"{base_sha}...{head_sha}"], ROOT, timeout=120)
    if p.returncode != 0:
        tail = ((p.stderr or "") + "\n" + (p.stdout or "")).strip().splitlines()
        hint = tail[-1][:160] if tail else f"rc={p.returncode}"
        raise OSError(f"cannot diff {base_sha[:7]}...{head_sha[:7]}: {hint}")
    out = []
    for line in (p.stdout or "").splitlines():
        name = line.strip()
        if not name:
            continue
        if name.startswith("crates/") or name in ("Cargo.toml", "Cargo.lock"):
            out.append(name)
    return sorted(out)


def golden_tests_pass(timeout: int = 900) -> "tuple[bool, str]":
    """Golden tests byte-identical to what is committed (AC4 condition 3)."""
    try:
        p = subprocess.run(
            ["cargo", "test", "--locked", "-p", "universe-app", "--test", "golden"],
            capture_output=True, text=True, encoding="utf-8",
            errors="replace", cwd=str(ROOT), timeout=timeout)
    except (OSError, subprocess.SubprocessError) as exc:
        return False, f"golden tests did not start: {exc}"
    except Exception as exc:
        return False, f"golden tests did not complete: {exc}"
    tail = ((p.stdout or "") + "\n" + (p.stderr or "")).strip().splitlines()
    lines = [ln for ln in tail if "test result" in ln]
    summary = lines[-1][:160] if lines else f"rc={p.returncode}"
    if p.returncode == 0:
        return True, f"golden tests rc=0 ({summary})"
    return False, f"golden tests rc={p.returncode} ({summary})"


def run_degraded(issue: int, sha: str, visual: str, base_desc: str,
                 base_sha: str, out_dir: Path, cause: str) -> int:
    """Degraded picture proof for an infra-only failure (issue #198, AC4).

    Reached only via `--simulate-infra <cause>` (or a real infra
    failure routed here): disk full, a locked exe, a failed upload.
    None of those say anything about the code, so when all four hold
    the round may merge with the proof recorded as blocked and the
    reason named — and with no before/after pictures for that round.
    When any one fails, this refuses and the full validation is
    required as before. Either way the exit is 2 (BLOCKED contract).
    """
    try:
        out_dir.mkdir(parents=True, exist_ok=True)
    except OSError as exc:
        print(f"FRAME-PROOF-BLOCKED cannot create {out_dir}: {exc}")
        return 2
    try:
        changed = capturable_changed_files(base_sha, sha)
    except OSError as exc:
        print(f"FRAME-PROOF-BLOCKED degraded refused: {exc}")
        return 2
    if changed:
        print(f"FRAME-PROOF-BLOCKED degraded refused: "
              f"{len(changed)} capturable file(s) changed "
              f"({', '.join(changed[:5])})")
        return 2
    goldens_ok, golden_note = golden_tests_pass()
    if not goldens_ok:
        print(f"FRAME-PROOF-BLOCKED degraded refused: {golden_note}")
        return 2
    b7, a7 = short_sha(base_sha), short_sha(sha)
    try:
        diff_names = _git(["diff", "--name-only", f"{base_sha}...{sha}"],
                          ROOT, timeout=120)
        total = len([ln for ln in (diff_names.stdout or "").splitlines()
                     if ln.strip()])
    except (OSError, subprocess.SubprocessError):
        total = 0
    fragment = (
        f"\nVisual: BLOCKED (infra: {cause}) \u2014 "
        f"<no capturable byte changed: base {b7}...head {a7}: "
        f"{total} file(s) differ, none under crates/, Cargo.toml, or Cargo.lock>\n"
        f"\n- infra cause: {cause}\n"
        f"- capturable files changed: none ({golden_note})\n"
        f"- golden tests: PASS byte-identical\n"
        f"\n<sub>Before {base_sha} / after {sha} "
        f"(degraded: no pictures published for this round)</sub>\n"
    )
    try:
        (out_dir / "frame-proof.md").write_text(fragment, encoding="utf-8")
        import json as _json

        (out_dir / "frame-proof.json").write_text(
            _json.dumps({"issue": issue, "before_sha": base_sha,
                         "after_sha": sha, "visual": visual,
                         "changed": [], "all_levels": [],
                         "dry_run": False, "release": None,
                         "urls": {}, "degraded": cause}, indent=2) + "\n",
            encoding="utf-8")
    except OSError as exc:
        print(f"FRAME-PROOF-BLOCKED write {exc}")
        return 2
    print(f"FRAME-PROOF-BLOCKED Visual: BLOCKED (infra: {cause})")
    print(f"degraded-accepted {cause} no-capturable-change goldens-pass")
    return 2


def before_cache_dir(base_sha: str) -> Path:
    """Cache dir for before-side frames from one base commit (AC6).

    Keyed by the git base SHA only, never by the exe digest: finding F1
    on #198 showed two captures of identical source produce different
    bytes, so a fingerprint comparison would report every warm run as
    cold. A moved base is a different key and captures fresh.
    Lives under the gitignored `.agent/cache/` with the role target
    dirs (AC7).
    """
    return ROOT / ".agent" / "cache" / "frame-proof-before" / base_sha


def cached_before_frames(base_sha: str) -> "dict[str, Path]":
    """Before frames cached for base_sha, empty when there are none."""
    d = before_cache_dir(base_sha)
    if not d.is_dir():
        return {}
    try:
        return {level_key(p.name): p for p in sorted(d.glob("frame-*.ppm"))
                if p.is_file()}
    except OSError:
        return {}


def store_before_frames(base_sha: str, before_dir: Path) -> None:
    """Best-effort: remember this run's before frames for base_sha."""
    try:
        dest = before_cache_dir(base_sha)
        dest.mkdir(parents=True, exist_ok=True)
        for p in before_dir.glob("frame-*.ppm"):
            if p.is_file():
                (dest / p.name).write_bytes(p.read_bytes())
    except OSError:
        pass


def run_captured(issue: int, sha: str, visual: str, base_arg: "str | None",
                 out_dir_arg: "Path | None", dry_run: bool,
                 simulate_infra: "str | None" = None) -> int:
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
    wt_dir = base_worktree_dir()
    try:
        out_dir.mkdir(parents=True, exist_ok=True)
    except OSError as exc:
        print(f"FRAME-PROOF-BLOCKED cannot create {out_dir}: {exc}")
        return 2

    if simulate_infra is not None:
        return run_degraded(issue, sha, visual, base_desc, base_sha,
                            out_dir, simulate_infra)

    after_target = shared_target_dir("after")
    ok, piece, rc, out, err = run_capture(ROOT, after_dir, "after", after_target)
    (out_dir / "capture-after.log").write_text(
        f"$ cargo run --locked -p universe-app -- --capture {after_dir}\n"
        f"{build_identity(ROOT, after_target)}\n"
        f"rc={rc}\n---stdout---\n{out}\n---stderr---\n{err}",
        encoding="utf-8")
    if not ok:
        print(f"FRAME-PROOF-BLOCKED {piece}")
        return 2

    _git(["worktree", "remove", "--force", str(wt_dir)], ROOT, timeout=120)
    import shutil as _shutil

    # AC6: reuse the previous proof's before-side frames when the base
    # has not moved. The cache key is the git base SHA (never the exe
    # digest, per F1). A moved base, an empty cache, or a frame set that
    # no longer matches the after side all capture fresh.
    after_frames = frame_files(after_dir)
    reuse = cached_before_frames(base_sha)
    if reuse and set(reuse) == set(after_frames):
        try:
            before_dir.mkdir(parents=True, exist_ok=True)
            for key, src in reuse.items():
                (before_dir / src.name).write_bytes(src.read_bytes())
        except OSError as exc:
            print(f"FRAME-PROOF-BLOCKED reuse before {base_sha[:7]}: {exc}")
            return 2
        (out_dir / "capture-before.log").write_text(
            f"base-reused {base_sha} ({len(reuse)} frames, from cache)\n"
            f"cwd=(cache) base={base_desc} {base_sha}\n",
            encoding="utf-8")
        print(f"base-reused {base_sha} ({len(reuse)} frames)")
        return run_offline(before_dir, after_dir, out_dir, issue,
                           base_sha, sha, visual, dry_run)

    _shutil.rmtree(wt_dir, ignore_errors=True)
    _git(["worktree", "prune"], ROOT, timeout=120)
    p = _git(["worktree", "add", "--detach", str(wt_dir), base_sha], ROOT, timeout=120)
    if p.returncode != 0:
        tail = ((p.stderr or "") + "\n" + (p.stdout or "")).strip().splitlines()
        hint = tail[-1][:200] if tail else f"rc={p.returncode}"
        print(f"FRAME-PROOF-BLOCKED worktree add {base_sha[:7]}: {hint}")
        return 2
    try:
        before_target = shared_target_dir("before")
        ok, piece, rc, out, err = run_capture(wt_dir, before_dir, "before", before_target)
        (out_dir / "capture-before.log").write_text(
            f"$ cargo run --locked -p universe-app -- --capture {before_dir}\n"
            f"cwd={wt_dir} base={base_desc} {base_sha}\n"
            f"{build_identity(wt_dir, before_target)}\n"
            f"rc={rc}\n---stdout---\n{out}\n---stderr---\n{err}",
            encoding="utf-8")
        if not ok:
            print(f"FRAME-PROOF-BLOCKED {piece}")
            return 2
        store_before_frames(base_sha, before_dir)
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
    p.add_argument("--simulate-infra", default=None,
                   choices=["disk-full", "locked-exe", "upload-fail"],
                   help="captured mode: fail the capture for an infrastructure "
                        "reason only, without touching disk, processes, or the "
                        "network (issue #198, AC4 test affordance)")
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
                args.simulate_infra,
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
