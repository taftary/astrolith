"""Fixed-suite evidence handling; no caller-supplied executable or command."""

from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import subprocess
import sys

from build_receipt import verify
from profile_process import observe, write_json
from profile_run import APP, ROOT, RunLock, fingerprint, git, player_settings
from scenario_profile import outside_real

SCENARIOS = ROOT / "scripts/validation/scenarios"
PROFILES = ROOT / "scripts/validation/profiles"
PROBLEMS = re.compile(r"\b(error|warn|panic|deprecated|FAILED)\b", re.I)
ACCEPTED_VULKAN = ("VUID-vkAcquireNextImageKHR-semaphore-01286", "VUID-VkPresentInfoKHR-pImageIndices-01430")


def read_json(path):
    if path.stat().st_size > 16 * 1024 * 1024:
        raise ValueError("suite JSON exceeds 16 MiB")
    return json.loads(path.read_text(encoding="utf-8"))


def diagnostics(paths):
    """Keep every matched line, separating the two owner-accepted Vulkan IDs."""
    result = {"unexpected": [], "owner_accepted": []}
    for path in paths:
        if not path.is_file():
            continue
        if path.stat().st_size > 16 * 1024 * 1024:
            raise ValueError("suite log exceeds 16 MiB")
        for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            if PROBLEMS.search(line):
                key = "owner_accepted" if any(code in line for code in ACCEPTED_VULKAN) else "unexpected"
                result[key].append({"file": str(path), "line": number, "text": line})
    return result


def scenario_result(code, manifest, reason_text, expected_reason):
    success = expected_reason is None
    return (code == (0 if success else 1)
            and (manifest is None or (manifest.get("complete") is success
                                     and manifest.get("settings_unchanged") is True))
            and (manifest is not None if success else expected_reason in reason_text))


class Suite:
    def __init__(self, out, receipt, group):
        real_root = player_settings().parent.parent.resolve()
        self.out = outside_real(out, real_root)
        self.receipt = outside_real(receipt, real_root)
        if git("status", "--porcelain", "--untracked-files=no"):
            raise ValueError("commit tracked source before running the capability suite")
        self.build = verify(self.receipt, APP, git("rev-parse", "HEAD"))
        self.out.mkdir(parents=True, exist_ok=False)
        self.record = {"schema": "moon-validation-suite-v1", "complete": False, "group": group,
                       "started_utc": datetime.now(timezone.utc).isoformat(), "build_receipt": self.build,
                       "settings_before": fingerprint(player_settings()), "results": [],
                       "limitations": ["execution evidence, not an independent verdict by itself",
                                       "preparation is not travel; moon hovering is not landing",
                                       "deliberately corrupted copies are parser tests, not observed app failures"]}
        write_json(self.out / "suite.json", self.record)

    def save(self):
        self.record["settings_after"] = fingerprint(player_settings())
        self.record["settings_unchanged"] = self.record["settings_before"] == self.record["settings_after"]
        write_json(self.out / "suite.json", self.record)

    def record_result(self, row):
        self.record["results"].append(row)
        self.save()
        print(json.dumps({"case": row["name"], "as_expected": row["as_expected"], "out": str(self.out)}), flush=True)
        if not row["as_expected"] or not self.record["settings_unchanged"]:
            raise ValueError(f"suite stopped on unexpected result: {row['name']}")

    def command(self, name, script, arguments, expected=0):
        # Only fixed repository entry points selected by this module's callers.
        if script not in ("scenario_run.py", "scenario_persistence.py", "scenario_comparison.py", "scenario_audit.py", "profile_run.py", "scenario_catalog.py"):
            raise ValueError("not a fixed suite entry point")
        argv = [sys.executable, str(ROOT / "scripts/validation" / script), *map(str, arguments)]
        stdout, stderr = self.out / f"{name}-runner.stdout.txt", self.out / f"{name}-runner.stderr.txt"
        with stdout.open("xb") as output, stderr.open("xb") as problems:
            result = subprocess.run(argv, cwd=ROOT, stdout=output, stderr=problems, check=False)
        scan = diagnostics([stdout, stderr])
        return {"name": name, "argv": argv, "exit_code": result.returncode,
                "expected_exit_code": expected, "as_expected": result.returncode == expected and not scan["unexpected"],
                "runner_stdout": str(stdout), "runner_stderr": str(stderr), "log_scan": scan}

    def scenario(self, name, fixture, *, profile=None, seed=None, collection=None,
                 reason=None, timeout=180, extra=(), receipt=None, target=None):
        target = target or self.out / name
        arguments = ["--scenario", SCENARIOS / f"{fixture}.scenario", "--out", target,
                     "--timeout", timeout, "--build-receipt", receipt or self.receipt]
        if profile is not None:
            arguments += ["--profile", profile]
        if seed is not None:
            arguments += ["--profile-fixture", PROFILES / f"{seed}.toml"]
        if collection is not None:
            arguments.append(collection)
        arguments += list(extra)
        row = self.command(name, "scenario_run.py", arguments, 0 if reason is None else 1)
        manifest_path = target / "manifest.json"
        manifest = read_json(manifest_path) if manifest_path.is_file() else None
        logs = [Path(row["runner_stdout"]), Path(row["runner_stderr"]), target / "stdout.txt", target / "stderr.txt"]
        stdout = (target / "stdout.txt").read_text(encoding="utf-8") if (target / "stdout.txt").is_file() else ""
        reason_text = stdout + Path(row["runner_stdout"]).read_text(encoding="utf-8") + json.dumps(manifest)
        row.update(manifest=str(manifest_path) if manifest is not None else None,
                   expected_reason_fragment=reason, terminal=stdout.splitlines()[-1:] or None,
                   problems=manifest.get("problems") if manifest is not None else None, log_scan=diagnostics(logs))
        row["as_expected"] &= scenario_result(row["exit_code"], manifest, reason_text, reason)
        row["as_expected"] &= not row["log_scan"]["unexpected"]
        if manifest and manifest.get("executable_sha256"):
            row["as_expected"] &= manifest["executable_sha256"] == self.build["executable_sha256"]
        self.record_result(row)
        return target

    def compare(self, name, before, restart, recalls=False):
        args = ["--before", before, "--restart", restart, "--out", self.out / name]
        if recalls:
            args.append("--recalls")
        self.record_result(self.command(name, "scenario_persistence.py", args))

    def normal(self, switches=False):
        """Bounded ordinary launch with environment-only switches; not a Quit test."""
        name = "normal-environment-only" if switches else "normal-startup"
        target = self.out / name
        target.mkdir()
        profile = target / "profile"
        profile.mkdir()
        (profile / ".astrolith-profile").write_bytes(b"disposable-v1\n")
        environment = os.environ.copy()
        environment.update(APPDATA=str(profile), RUST_BACKTRACE="1")
        for key in ("ASTROLITH_PROFILE", "ASTROLITH_GPU_TIMERS"):
            if switches:
                environment[key] = "1"
            else:
                environment.pop(key, None)
        environment.pop("ASTROLITH_PROTOTYPE", None)
        environment.setdefault("VK_IMPLICIT_LAYER_PATH", str(ROOT / ".cargo"))
        environment.setdefault("VK_LOADER_LAYERS_DISABLE", "~implicit~")
        argv = [str(APP), "--size", "1600x900", "--build-workers", "5"]
        with RunLock():
            process = observe(argv, target, ROOT, environment, 10, collect=False)
        logs = diagnostics([target / "stdout.txt", target / "stderr.txt"])
        stderr = (target / "stderr.txt").read_text(encoding="utf-8")
        forbidden = {"input-measure-v1.tsv", "loading-v1.tsv", "scenario.tsv", "telemetry-v1.csv"}
        generated = [str(path) for path in target.rglob("*") if path.name in forbidden]
        self.record_result({"name": name, "argv": argv, "process": process,
                            "environment_switches": {key: environment.get(key) for key in ("ASTROLITH_PROFILE", "ASTROLITH_GPU_TIMERS")},
                            "log_scan": logs, "test_sidecars": generated,
                            "expected_termination": "bounded-timeout-not-normal-Quit",
                            "as_expected": process["status"] == "timeout" and "AdapterInfo" in stderr
                            and "Creating new window" in stderr and not generated and not logs["unexpected"]})

    def finish(self):
        self.record["completed_utc"] = datetime.now(timezone.utc).isoformat()
        self.save()
        self.record["complete"] = (bool(self.record["results"]) and self.record["settings_unchanged"]
                                   and all(row["as_expected"] for row in self.record["results"]))
        if git("status", "--porcelain", "--untracked-files=no"):
            raise ValueError("tracked source changed during the capability suite")
        verify(self.receipt, APP, git("rev-parse", "HEAD"))
        self.save()
        return 0 if self.record["complete"] else 1
