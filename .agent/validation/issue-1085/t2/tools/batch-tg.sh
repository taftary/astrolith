#!/bin/sh
cd /c/Users/PERSO/source/repos/astrolith || exit 1
LOG="$1"; D="$2"
O=.agent/validation/issue-1085/t2/toggles
run() {
  r=$(python scripts/validation/profile_run.py --route "$D/$1.route" --out "$O/$1" --timeout 900 --workers 5 --width 1600 --height 900 --refresh-hz 60 --power-note "Balanced 381b4222, AC present, observed 2026-11-03" 2>&1 | tail -1)
  echo "$(date -u +%H:%M:%S) $1 $r" >> "$LOG"
}
export ASTROLITH_GPU_TIMERS=1
for t in tg-base tg-atmo-low tg-atmo-high tg-shadows tg-bloom tg-shadows-bloom tg-cap30 tg-capped; do run $t; done
echo "$(date -u +%H:%M:%S) BATCH-DONE" >> "$LOG"
