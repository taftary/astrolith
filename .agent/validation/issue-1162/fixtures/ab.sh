#!/usr/bin/env bash
# #1162 T7: ABBA paired repeats on the short route (before, after, after,
# before, ...), so a slow drift of the machine cancels out between builds.
# Usage (repo root): bash .agent/validation/issue-1162/fixtures/ab.sh <before.exe> <after.exe> <out dir> [rounds] [route]
set -u
before="$1"; after="$2"; out="$3"; rounds="${4:-2}"; route="${5:-ab-short}"
D=.agent/validation/issue-1162
mkdir -p "$out"
sha256sum "$before" "$after" > "$out/exe.sha256"
n=0
run() { # side
  n=$((n + 1))
  cp "$([ "$1" = before ] && echo "$before" || echo "$after")" target/release/universe-app.exe
  python scripts/validation/profile_run.py --route "$D/fixtures/$route.route" --out "$out/$n-$1" --timeout 1200 --power-note "AC, Balanced" > "$out/$n-$1.log" 2>&1
  echo "$n $1 rc=$?"
}
for r in $(seq "$rounds"); do run before; run after; run after; run before; done
cp "$after" target/release/universe-app.exe
echo AB-DONE
