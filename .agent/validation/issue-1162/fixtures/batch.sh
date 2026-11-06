#!/usr/bin/env bash
# #1162: matched pictures plus profile routes for one side (before or after).
# Usage (repo root): bash .agent/validation/issue-1162/fixtures/batch.sh <before|after> <exe>
# The exe is copied to target/release/universe-app.exe, which profile_run.py runs.
set -u
side="$1"
exe="$2"
D=.agent/validation/issue-1162
out="$D/$side"
mkdir -p "$out"
[ "$exe" -ef target/release/universe-app.exe ] || cp "$exe" target/release/universe-app.exe
sha256sum target/release/universe-app.exe > "$out/exe.sha256"
git rev-parse HEAD > "$out/commit.txt"
rm -rf "$out/pictures"
./target/release/universe-app.exe --screens "$out/pictures" --shots "$D/fixtures/water.shots" > "$out/pictures.log" 2>&1
echo "pictures rc=$? $(tail -1 "$out/pictures.log")"
for r in water-fixed water-terrain-low water-terrain-high water-auto; do
  python scripts/validation/profile_run.py --route "$D/fixtures/$r.route" --out "$out/$r" --timeout 1200 --power-note "AC, Balanced" > "$out/$r.log" 2>&1
  echo "$r rc=$?"
done
echo BATCH-DONE
