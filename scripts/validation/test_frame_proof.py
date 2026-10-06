"""Fixture harness for frame_proof.py core (issue #134, T1).

Canned PPM pairs plus real capture dirs, no network: PNG bytes decode,
identical dirs compare clean, one differing frame names that level only,
fragment layout holds, and the visual/missing exit codes apply.

Usage:
  python scripts/validation/test_frame_proof.py
Exit 0 when every fixture behaves; non-zero naming the first failure.
"""

import os
import shutil
import struct
import subprocess
import sys
import tempfile
import zlib
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from frame_proof import (  # noqa: E402
    RELEASE_NAME,
    RELEASE_TAG,
    asset_name,
    app_exe,
    base_worktree_dir,
    build_fragment,
    build_identity,
    compare_dirs,
    encode_png,
    git_rev_parse,
    level_key,
    main as fp_main,
    parse_ppm,
    placeholder_url,
    resolve_base_sha,
    run_captured,
    run_offline,
    scale_2x,
    shared_target_dir,
)

REAL_CAPTURE = Path(os.environ.get("FRAME_PROOF_FIXTURE", ""))


def check(name, got, want):
    if got != want:
        print(f"fixture {name}: MISMATCH (got {got!r}, want {want!r})")
        return False
    print(f"fixture {name}: ok")
    return True


def write_ppm(path: Path, width: int, height: int, pixels: list[tuple[int, int, int]]):
    lines = ["P3", f"# test {path.name}", f"{width} {height}", "255"]
    row: list[str] = []
    for r, g, b in pixels:
        row += [str(r), str(g), str(b)]
    lines.append(" ".join(row) + " ")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def png_size(png: bytes) -> tuple[int, int]:
    if png[:8] != b"\x89PNG\r\n\x1a\n":
        raise AssertionError("missing PNG signature")
    pos = 8
    while pos < len(png):
        (length,) = struct.unpack(">I", png[pos:pos + 4])
        ctype = png[pos + 4:pos + 8]
        data = png[pos + 8:pos + 8 + length]
        if ctype == b"IHDR":
            w, h = struct.unpack(">II", data[:8])
            return w, h
        pos += 12 + length
    raise AssertionError("no IHDR found")


def png_idat(png: bytes) -> bytes:
    pos = 8
    parts = bytearray()
    while pos < len(png):
        (length,) = struct.unpack(">I", png[pos:pos + 4])
        ctype = png[pos + 4:pos + 8]
        data = png[pos + 8:pos + 8 + length]
        if ctype == b"IDAT":
            parts += data
        pos += 12 + length
    return zlib.decompress(bytes(parts))


def fresh_dir(root: Path, name: str) -> Path:
    d = root / name
    if d.exists():
        shutil.rmtree(d)
    d.mkdir(parents=True)
    return d


def main() -> int:
    ok = True
    root = Path(tempfile.mkdtemp(prefix="frame-proof-test-"))
    try:
        # 1. PPM parse + 2x PNG encode validity.
        ppm = root / "small.ppm"
        write_ppm(ppm, 2, 1, [(255, 0, 0), (0, 0, 0)])
        w, h, rgb = parse_ppm(ppm)
        ok &= check("parse-size", (w, h, len(rgb)), (2, 1, 6))
        w2, h2, big = scale_2x(w, h, rgb)
        ok &= check("scale-size", (w2, h2, len(big)), (4, 2, 24))
        png = encode_png(w2, h2, big)
        ok &= check("png-size", png_size(png), (4, 2))
        raw = png_idat(png)
        # First scanline: filter 0 + red red black black at 2x.
        ok &= check("png-row0", list(raw[0:13]),
                    [0, 255, 0, 0, 255, 0, 0, 0, 0, 0, 0, 0, 0])

        # 2. Identical dirs -> 0 changed; level keys sorted L1..L11 style.
        a = fresh_dir(root, "a")
        b = fresh_dir(root, "b")
        (a / "frame-L1-Alpha.ppm").write_bytes(ppm.read_bytes())
        (a / "frame-L2-Beta.ppm").write_bytes(ppm.read_bytes())
        (b / "frame-L1-Alpha.ppm").write_bytes(ppm.read_bytes())
        (b / "frame-L2-Beta.ppm").write_bytes(ppm.read_bytes())
        changed, levels = compare_dirs(a, b)
        ok &= check("identical-changed", changed, [])
        ok &= check("identical-levels", levels, ["L1-Alpha", "L2-Beta"])
        ok &= check("level-key", level_key("frame-L10-Stars.ppm"), "L10-Stars")

        # 3. One differing frame -> that level only.
        other = root / "other.ppm"
        write_ppm(other, 2, 1, [(0, 255, 0), (0, 0, 0)])
        (b / "frame-L2-Beta.ppm").write_bytes(other.read_bytes())
        changed, _ = compare_dirs(a, b)
        ok &= check("one-diff", changed, ["L2-Beta"])

        # 4. Fragment layout: blank first line, Visual line, pair row
        #    with both SHAs, details block with every after frame.
        frag = build_fragment(
            ["L2-Beta"], ["L1-Alpha", "L2-Beta"],
            "a" * 40, "b" * 40,
            lambda kind, level: placeholder_url(134, "a" * 40 if kind == "before" else "b" * 40, kind, level),
        )
        ok &= check("fragment-blank-first", frag.startswith("\n"), True)
        ok &= check("fragment-visual", "Visual:" in frag, True)
        ok &= check("fragment-pair", "| L2-Beta |" in frag, True)
        ok &= check("fragment-shas", ("a" * 40 in frag) and ("b" * 40 in frag), True)
        ok &= check("fragment-details", "<details>" in frag and "</details>" in frag, True)
        ok &= check("fragment-all-after", frag.count("![after ") >= 3, True)
        calm = build_fragment([], ["L1-Alpha", "L2-Beta"], "a" * 40, "b" * 40,
                              lambda kind, level: placeholder_url(134, "a" * 40, kind, level))
        ok &= check("fragment-calm-one-line",
                    [ln for ln in calm.splitlines() if ln.startswith("Visual:")],
                    ["Visual: none (2 frames identical to main)"])

        # 5. Exit codes via run_offline on canned dirs.
        out = fresh_dir(root, "out")
        rc = run_offline(a, b, out, 134, "a" * 40, "b" * 40, "no", True)
        ok &= check("offline-diff-no-visual-rc", rc, 1)
        ok &= check("offline-md-written", (out / "frame-proof.md").is_file(), True)
        ok &= check("offline-json-written", (out / "frame-proof.json").is_file(), True)
        out2 = fresh_dir(root, "out2")
        c = fresh_dir(root, "c")
        (c / "frame-L1-Alpha.ppm").write_bytes(ppm.read_bytes())
        (c / "frame-L2-Beta.ppm").write_bytes(ppm.read_bytes())
        rc = run_offline(a, c, out2, 134, "a" * 40, "b" * 40, "yes", True)
        ok &= check("offline-visual-yes-clean-rc", rc, 1)
        out3 = fresh_dir(root, "out3")
        rc = run_offline(a, c, out3, 134, "a" * 40, "b" * 40, "no", True)
        ok &= check("offline-clean-rc", rc, 0)
        out4 = fresh_dir(root, "out4")
        rc = run_offline(root / "does-not-exist", c, out4, 134, "a" * 40, "b" * 40, "no", True)
        ok &= check("offline-missing-rc", rc, 2)

        # 6. CLI smoke: --visual yes on identical dirs exits 1 with the
        #    machine line; missing dir exits 2. Captured text carries none
        #    of the reserved vocabulary words.
        p = subprocess.run(
            [sys.executable, str(HERE / "frame_proof.py"),
             "--before-dir", str(a), "--after-dir", str(c),
             "--out-dir", str(fresh_dir(root, "cli")),
             "--issue", "134", "--before-sha", "a" * 40, "--after-sha", "b" * 40,
             "--visual", "yes", "--dry-run"],
            capture_output=True, text=True, cwd=str(HERE.parent.parent))
        ok &= check("cli-mismatch-rc", p.returncode, 1)
        ok &= check("cli-mismatch-line", "FRAME-PROOF-MISMATCH" in p.stdout, True)
        p = subprocess.run(
            [sys.executable, str(HERE / "frame_proof.py"),
             "--before-dir", str(root / "does-not-exist"), "--after-dir", str(c),
             "--out-dir", str(fresh_dir(root, "cli2")),
             "--issue", "134", "--before-sha", "a" * 40, "--after-sha", "b" * 40,
             "--visual", "no", "--dry-run"],
            capture_output=True, text=True, cwd=str(HERE.parent.parent))
        ok &= check("cli-blocked-rc", p.returncode, 2)
        ok &= check("cli-blocked-line", "FRAME-PROOF-BLOCKED" in p.stdout, True)
        blob = (p.stdout + "\n" + (out3 / "frame-proof.md").read_text(encoding="utf-8")).lower()
        for word in ("error", "warn", "panic", "deprecated"):
            ok &= check(f"vocab-{word}", word not in blob, True)
        ok &= check("vocab-failed", "FAILED" in (p.stdout + (out3 / "frame-proof.md").read_text(encoding="utf-8")), False)

        # 7. Real capture dirs when provided (E-GOLDEN untouched: read-only).
        if REAL_CAPTURE.is_dir() and list(REAL_CAPTURE.glob("frame-*.ppm")):
            before = fresh_dir(root, "real-before")
            after = fresh_dir(root, "real-after")
            for f in REAL_CAPTURE.glob("frame-*.ppm"):
                shutil.copy(f, before / f.name)
                shutil.copy(f, after / f.name)
            changed, levels = compare_dirs(before, after)
            ok &= check("real-identical", changed, [])
            ok &= check("real-count", len(levels), 11)
            victim = sorted(after.glob("frame-*.ppm"))[0]
            data = victim.read_bytes() + b" "
            victim.write_bytes(data)
            changed, _ = compare_dirs(before, after)
            ok &= check("real-one-diff", changed, [level_key(victim.name)])
            outr = fresh_dir(root, "real-out")
            rc = run_offline(before, after, outr, 134, "a" * 40, "b" * 40, "no", True)
            ok &= check("real-offline-mismatch-rc", rc, 1)
            shutil.copy(before / victim.name, after / victim.name)
            outr2 = fresh_dir(root, "real-out2")
            rc = run_offline(before, after, outr2, 134, "a" * 40, "b" * 40, "no", True)
            ok &= check("real-offline-rc", rc, 0)
            first_png = next(outr2.glob("after-*.png"))
            ok &= check("real-png-size", png_size(first_png.read_bytes()), (640, 400))
        # 8. T2 base resolution + sha gate (fast paths only: no cargo run).
        import re as _re

        head = git_rev_parse("HEAD")
        ok &= check("t2-head-hex", bool(_re.fullmatch(r"[0-9a-f]{40}", head)), True)
        desc, sha = resolve_base_sha("HEAD")
        ok &= check("t2-base-arg", (desc, sha), ("HEAD", head))
        desc, sha = resolve_base_sha(None)
        ok &= check("t2-base-desc", desc in ("merge-base HEAD origin/main", "HEAD~1"), True)
        ok &= check("t2-base-hex", bool(_re.fullmatch(r"[0-9a-f]{40}", sha)), True)
        outm = fresh_dir(root, "mismatch")
        rc = run_captured(134, "0" * 40, "no", None, outm, True)
        ok &= check("t2-sha-gate-rc", rc, 2)
        ok &= check("t2-sha-gate-no-wt", (outm / "base-wt").exists(), False)
        ok &= check("t2-base-wt-stable", base_worktree_dir(), base_worktree_dir())
        ok &= check("t2-base-wt-outside-repo",
                    Path(str(base_worktree_dir())).is_relative_to(Path.cwd()), False)
        rc = fp_main(["--issue", "134", "--sha", "0" * 40,
                      "--visual", "no", "--dry-run",
                      "--out-dir", str(fresh_dir(root, "mismatch-cli"))])
        ok &= check("t2-cli-sha-gate-rc", rc, 2)

        # 9. T3 naming and release constants (pure; the upload path needs
        #    the network and is proven by the manual run on the Issue).
        ok &= check("t3-tag", RELEASE_TAG, "validation-evidence")
        ok &= check("t3-name", RELEASE_NAME, "Validation evidence (do not use)")
        ok &= check("t3-asset-before",
                    asset_name(134, "a" * 40, "before", "L1-Alpha"),
                    "issue-134-aaaaaaa-before-L1-Alpha.png")
        ok &= check("t3-asset-after",
                    asset_name(134, "b" * 40, "after", "L10-Stars"),
                    "issue-134-bbbbbbb-after-L10-Stars.png")
        ok &= check("t3-target-stable", shared_target_dir("after"), shared_target_dir("after"))
        ok &= check("t3-target-split", shared_target_dir("after") == shared_target_dir("before"), False)
        # The role caches live in the repo, under the gitignored .agent/cache,
        # because OS temp cleanup used to delete them and each deletion cost a
        # cold rebuild of both sides (~26 min, ~10 GB). They must stay out of
        # the workspace source tree, out of version control, and inside the
        # repo (not temp) at the same time.
        after_cache = shared_target_dir("after")
        ok &= check("t3-target-in-repo", after_cache.is_relative_to(Path.cwd()), True)
        ok &= check("t3-target-under-agent-cache",
                    after_cache.is_relative_to(Path.cwd() / ".agent" / "cache"), True)
        ok &= check("t3-target-outside-source",
                    after_cache.is_relative_to(Path.cwd() / "crates"), False)
        ok &= check("t3-target-not-in-temp",
                    after_cache.is_relative_to(Path(tempfile.gettempdir())), False)
        ignored = subprocess.run(["git", "check-ignore", "-q", str(after_cache)],
                                  capture_output=True, text=True)
        ok &= check("t3-target-gitignored", ignored.returncode, 0)
        # Hermetic: the helpers must handle an empty cache. The real role
        # dir may hold a warm exe on the dev PC, so probe a fresh temp
        # dir, never machine-local state (E-TEST-HERMETIC).
        empty_cache = fresh_dir(root, "empty-target")
        ok &= check("t3-exe-none-in-fresh-cache", app_exe(empty_cache), None)
        ok &= check("t3-identity-no-exe",
                    "exe=(none)" in build_identity(Path.cwd(), empty_cache), True)
    finally:
        shutil.rmtree(root, ignore_errors=True)
    if not ok:
        print("fixtures: MISMATCH")
        return 1
    print("fixtures: all ok (frame proof core)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
