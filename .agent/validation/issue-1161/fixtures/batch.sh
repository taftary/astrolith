#!/usr/bin/env bash
# #1161: matched pictures plus profile routes for one side (before or after).
# Usage: bash .agent/validation/issue-1161/fixtures/batch.sh <before|after>
# Needs a fresh `cargo build --locked --release -p universe-app`.
set -u
side="$1"
D=.agent/validation/issue-1161
out="$D/$side"
mkdir -p "$out"
sha256sum target/release/universe-app.exe > "$out/exe.sha256"
git rev-parse HEAD > "$out/commit.txt"
./target/release/universe-app.exe --list-worlds > "$out/worlds.txt" 2>&1
echo "worlds rc=$? $(tail -1 "$out/worlds.txt")"
rm -rf "$out/pictures"
./target/release/universe-app.exe --screens "$out/pictures" --shots "$D/fixtures/surfaces.shots" > "$out/pictures.log" 2>&1
echo "pictures rc=$? $(tail -1 "$out/pictures.log")"
for r in surfaces-fixed surfaces-terrain-low surfaces-terrain-high surfaces-auto; do
  python scripts/validation/profile_run.py --route "$D/fixtures/$r.route" --out "$out/$r" --timeout 1200 --power-note "AC, Balanced" > "$out/$r.log" 2>&1
  echo "$r rc=$?"
done
echo BATCH-DONE
