#!/usr/bin/env bash
# #1161 T8: orbit-only paired repeats, before build vs a candidate build,
# alternating, two rounds each.
# Usage: bash .agent/validation/issue-1161/fixtures/orbit-paired.sh <before.exe> <candidate.exe> <out dir>
set -u
before="$1"; cand="$2"; out="$3"
D=.agent/validation/issue-1161
mkdir -p "$out"
sha256sum "$before" "$cand" > "$out/exe.sha256"
for round in 1 2; do
  for side in before after; do
    cp "$([ "$side" = before ] && echo "$before" || echo "$cand")" target/release/universe-app.exe
    python scripts/validation/profile_run.py --route "$D/fixtures/surfaces-orbit.route" --out "$out/$side-surfaces-orbit-r$round" --timeout 900 --power-note "AC, Balanced" > "$out/$side-surfaces-orbit-r$round.log" 2>&1
    echo "$side r$round rc=$?"
  done
done
cp "$cand" target/release/universe-app.exe
echo ORBIT-DONE
