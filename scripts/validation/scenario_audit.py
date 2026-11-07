"""Recompute retained evidence and reject explicitly corrupted copies (#1238).

python scripts/validation/scenario_audit.py --run RUN --out NEW_DIRECTORY
python scripts/validation/scenario_audit.py --pair OFF1 ON1 --pair OFF2 ON2 --pair OFF3 ON3 --out NEW_DIRECTORY
Originals are read-only. Altered files are labeled parser tests, not app evidence.
"""

import argparse
import copy
import json
from pathlib import Path
import shutil
import sys

from build_receipt import verify_record
from profile_process import write_json
from profile_run import digest, player_settings
from scenario_comparison import adapter, native, overhead, read_run
from scenario_loading import summarize as loading_summary
from scenario_measure import read_frames, summarize
from scenario_measure_trace import InputTrace
from scenario_profile import outside_real, overlap
from scenario_suite_support import read_json
from scenario_timing import Timing, timing_summary

RAW = ("app/scenario.tsv", "app/input-measure-v1.tsv", "app/loading-v1.tsv", "memory.jsonl", "process.json",
       "manifest.json", "build-receipt.json", "fixture.scenario", "conditions.jsonl", "stdout.txt", "stderr.txt",
       "input-report.json", "timing-report.json", "loading-report.json")


def drop_line(text, fragment):
    rows = text.splitlines(keepends=True)
    index = next((i for i, row in enumerate(rows) if fragment in row), None)
    if index is None:
        raise ValueError("requested corruption has no original matching record")
    del rows[index]
    return "".join(rows)


def missing_reindexed_state(text):
    rows = drop_line(text, "\tkind=state\t").splitlines()
    count = 0
    for index, row in enumerate(rows):
        fields = row.split("\t")
        if row.startswith("MEASURE-EVENT\t"):
            rows[index] = "\t".join(f"sequence={count}" if field.startswith("sequence=") else field for field in fields)
            count += 1
        elif row.startswith("MEASURE-END\t"):
            rows[index] = "\t".join(f"records={count}" if field.startswith("records=") else field for field in fields)
    return "\n".join(rows) + "\n"


class Audit:
    def __init__(self, out, sources):
        real_root = player_settings().parent.parent
        self.out = outside_real(out, real_root)
        self.sources = [outside_real(source, real_root) for source in sources]
        if any(overlap(self.out, source) for source in self.sources):
            raise ValueError("audit output must be separate from every original evidence directory")
        self.originals = {}
        for source in self.sources:
            if not source.is_dir():
                raise ValueError("audit source must be an existing evidence directory")
            for relative in RAW:
                path = outside_real(source / relative, real_root)
                if path.exists():
                    if not path.is_file() or path.stat().st_size > 268435456:
                        raise ValueError("audit source file is not a bounded regular evidence file")
                    self.originals[str(path)] = digest(path)
        self.out.mkdir(parents=True, exist_ok=False)
        self.report = {"schema": "moon-validation-evidence-audit-v1", "complete": False,
                       "original_sha256": self.originals, "negative_copies": [],
                       "limitations": ["deliberate corrupted-copy parser checks, not observed application failures",
                                       "no arbitrary command, build or original-evidence edit"]}
        self.save()

    def save(self):
        self.report["original_evidence_unchanged"] = all(Path(path).is_file() and digest(path) == sha for path, sha in self.originals.items())
        write_json(self.out / "audit.json", self.report)

    def refuses(self, name, action):
        try:
            action()
        except (OSError, ValueError, KeyError, TypeError) as exc:
            self.report["negative_copies"].append({"name": name, "refused": True, "reason": str(exc)})
            self.save()
            return
        raise ValueError(f"corrupted copy was not refused: {name}")

    def changed_text(self, name, original, changed):
        if changed == original:
            raise ValueError(f"corruption did not change original bytes: {name}")
        path = self.out / f"deliberately-corrupted-{name}.tsv"
        path.write_bytes(changed.encode("utf-8"))
        return path

    def measurements(self):
        source = self.sources[0]
        manifest = read_json(source / "manifest.json")
        windows = manifest["measurement_windows"]
        actual = summarize(source, windows["warmup_s"], windows["scored_s"])
        if actual != read_json(source / "input-report.json"):
            raise ValueError("raw recomputation differs from retained input report")
        self.report["recomputed_identically"] = True
        write_json(self.out / "recomputed-input-report.json", actual)
        trace = InputTrace(source / "app/scenario.tsv")
        original = (source / "app/input-measure-v1.tsv").read_text(encoding="utf-8")
        for name, fragment in (("missing-state", "\tkind=state\t"), ("missing-main-work", "\tkind=main-work\t"),
                               ("missing-render-work", "\tkind=render-work\t"), ("missing-seal", "MEASURE-END\t")):
            path = self.changed_text(name, original, drop_line(original, fragment))
            self.refuses(name, lambda: read_frames(path, trace))
        original = (source / "app/loading-v1.tsv").read_text(encoding="utf-8")
        path = self.changed_text("missing-loading-event", original, drop_line(original, "LOAD-EVENT\t"))
        self.refuses("missing-loading-event", lambda: loading_summary(path))
        original = (source / "app/scenario.tsv").read_text(encoding="utf-8")
        for name, changed in (("unlinked-action", drop_line(original, "\tkind=ACTION")),
                              ("preparation-called-travel", original.replace("counts_as_travel=false", "counts_as_travel=true", 1))):
            path = self.changed_text(name, original, changed)
            self.refuses(name, lambda: InputTrace(path))
        for key, name in (("pid", "memory-owned-pid"), ("observed_peak_working_set_bytes", "memory-peak")):
            directory = self.out / f"deliberately-corrupted-{name}"
            (directory / "app").mkdir(parents=True)
            for relative in RAW[:5]:
                shutil.copyfile(source / relative, directory / relative)
            process = read_json(directory / "process.json")
            process[key] += 1
            write_json(directory / "process.json", process)
            self.refuses(name, lambda: summarize(directory, windows["warmup_s"], windows["scored_s"]))

    def pairs(self):
        runs = [read_run(source, "off" if index % 2 == 0 else "on") for index, source in enumerate(self.sources)]
        report = overhead(runs)
        if report != overhead([read_run(source, "off" if index % 2 == 0 else "on") for index, source in enumerate(self.sources)]):
            raise ValueError("raw comparison recomputation is not identical")
        if not report["overhead_within_limits"]:
            raise ValueError("actual matched overhead exceeds the declared limits")
        self.report["recomputed_identically"] = True
        write_json(self.out / "recomputed-overhead.json", report)
        source = self.sources[1]
        manifest = read_json(source / "manifest.json")
        receipt = read_json(source / "build-receipt.json")
        for key, value, name in (("executable_sha256", "0" * 64, "binary-hash"),
                                 ("source_commit_after", "0" * 40, "source-mismatch"),
                                 ("tracked_before", " M illustrative-change", "dirty-build")):
            path = self.out / f"deliberately-corrupted-{name}-receipt.json"
            write_json(path, {**receipt, key: value})
            self.refuses(name, lambda: verify_record(path, manifest["commit"], manifest["executable_sha256"], manifest["executable_bytes"]))
        conditions = [json.loads(line) for line in (source / "conditions.jsonl").read_text(encoding="utf-8").splitlines()]
        timing = timing_summary(source, 30, 120)
        actual_adapter = adapter(source / "stderr.txt")
        middle = next(i for i, row in enumerate(conditions) if row["utc_before_ns"] >= timing["score_utc_ns"][0] + 10_000_000_000)
        for name in ("missing-driver", "changed-refresh", "changed-power", "adapter-mismatch", "missing-native-row"):
            changed = copy.deepcopy(conditions)
            if name == "missing-driver":
                changed[middle]["display"]["driver"] = {"available": False, "reason": "deliberately removed from audit copy"}
            elif name == "changed-refresh":
                changed[middle]["display"]["refresh_hz"] += 1
            elif name == "changed-power":
                changed[middle]["power"]["ac_line"] = 1 - changed[middle]["power"]["ac_line"]
            elif name == "adapter-mismatch":
                changed[middle]["display"]["adapter_device_id"] = "PCI\\VEN_0000&DEV_0000"
            else:
                del changed[middle]
            directory = self.out / f"deliberately-corrupted-{name}"
            directory.mkdir()
            (directory / "conditions.jsonl").write_bytes(("\n".join(json.dumps(row) for row in changed) + "\n").encode())
            self.refuses(name, lambda: native(directory, manifest, timing, actual_adapter))
        original = (source / "app/input-measure-v1.tsv").read_text(encoding="utf-8")
        for name, changed in (("incomplete-seal", original.replace("complete=true", "complete=false")),
                              ("missing-frame-record", drop_line(original, "\tsequence=10\t"))):
            path = self.changed_text(name, original, changed)
            self.refuses(name, lambda: Timing(path))
        path = self.changed_text("reindexed-missing-full-state", original, missing_reindexed_state(original))
        self.refuses("reindexed-missing-full-state", lambda: read_frames(path, InputTrace(source / "app/scenario.tsv")))
        for name in ("unequal-source", "unequal-hash", "nonserial-lifetimes"):
            changed = copy.deepcopy(runs)
            if name == "unequal-source":
                changed[1]["identity"]["source_commit"] = "0" * 40
            elif name == "unequal-hash":
                changed[1]["identity"]["executable_sha256"] = "0" * 64
            else:
                changed[1]["started_utc"] = changed[0]["started_utc"]
            write_json(self.out / f"deliberately-corrupted-{name}.json", changed)
            self.refuses(name, lambda: overhead(changed))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--run", type=Path)
    mode.add_argument("--pair", type=Path, nargs=2, action="append", metavar=("OFF", "ON"))
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    audit = None
    try:
        if args.pair and len(args.pair) != 3:
            raise ValueError("exactly three pairs are required")
        audit = Audit(args.out, [args.run] if args.run else [path for pair in args.pair for path in pair])
        audit.measurements() if args.run else audit.pairs()
        expected = 9 if args.run else 14
        audit.report["complete"] = len(audit.report["negative_copies"]) == expected
        audit.save()
        audit.report["complete"] &= audit.report["original_evidence_unchanged"]
        audit.save()
        print(json.dumps({"complete": audit.report["complete"], "recomputed_identically": audit.report["recomputed_identically"],
                          "corrupted_copies_refused": len(audit.report["negative_copies"]), "out": str(audit.out)}))
        return 0 if audit.report["complete"] else 1
    except (OSError, ValueError, KeyError, TypeError, StopIteration) as exc:
        if audit is not None:
            audit.report.update(complete=False, problem=str(exc))
            audit.save()
        print(json.dumps({"complete": False, "reason": str(exc)}))
        return 1


if __name__ == "__main__":
    sys.exit(main())
