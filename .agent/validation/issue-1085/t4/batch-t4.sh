#!/bin/sh
# #1085 T4 (#1151): alternating baseline / inflight1 pairs, same executable.
cd /c/Users/PERSO/source/repos/astrolith || exit 1
LOG="$1"; F=scripts/validation/fixtures/profile; O=.agent/validation/issue-1085/t4
run() { # name fixture prototype
  if [ "$3" = none ]; then unset ASTROLITH_PROTOTYPE; else export ASTROLITH_PROTOTYPE=$3; fi
  r=$(python scripts/validation/profile_run.py --route "$F/$2.route" --out "$O/$1" --timeout 2400 --workers 5 --width 1600 --height 900 --refresh-hz 60 --power-note "Balanced 381b4222, AC present, observed 2026-11-03" 2>&1 | tail -1)
  echo "$(date -u +%H:%M:%S) $1 $r" >> "$LOG"
}
for k in 1 2 3; do
  for cfg in rank-normal rank-normal-uncapped rank-low rank-low-uncapped; do
    if [ $((k % 2)) = 1 ]; then
      run "$cfg-p$k-base" $cfg none; run "$cfg-p$k-proto" $cfg inflight1
    else
      run "$cfg-p$k-proto" $cfg inflight1; run "$cfg-p$k-base" $cfg none
    fi
  done
done
run sustained-base sustained-normal none
run sustained-proto sustained-normal inflight1
run transitions-proto transitions inflight1
run transitions-base transitions none
echo "$(date -u +%H:%M:%S) BATCH-DONE" >> "$LOG"
