"""Retain actual catalogs for the fixed checked-in moon fixtures; never a build.

python scripts/validation/scenario_catalog.py --build-receipt FILE --out NEW_DIRECTORY
No search for a universal minimum, generated replacement or expected property data.
"""

import argparse
import json
import math
from pathlib import Path
import subprocess
import sys

from build_receipt import verify
from profile_process import write_json
from profile_run import APP, ROOT, digest, fingerprint, git, player_settings
from scenario_profile import outside_real

FIXTURES = ("selected-moon", "moon-variety", "input-profile-moons")


def selections(text):
    result = []
    for line in text.splitlines():
        words = line.split("#", 1)[0].split()
        if words[:1] == ["setup"] and words[1:2] == ["moon"]:
            name, fields = "setup", words[2:]
        elif words[:1] == ["prepare"] and words[2:3] == ["moon"]:
            name, fields = words[1], words[3:]
        else:
            continue
        if len(fields) != 10:
            raise ValueError("checked-in moon fixture has an unsupported selected-start contract")
        host, marker, *parameters = fields
        result.append({"phase": name, "planet": host, "moon": marker,
                       "direction": parameters[:3], "gap_m": parameters[3], "game_s": parameters[4],
                       "mode": parameters[5], "yaw_deg": parameters[6], "pitch_deg": parameters[7],
                       "coordinate_basis": "moon-local-untranslated-host-cell-axes",
                       "placement_origin": "illustrative-preparation-not-travel"})
    return result


def parse_catalog(text, host):
    lines = text.splitlines()
    if not lines or not lines[-1].startswith("MOONS-OK "):
        raise ValueError("actual catalog did not complete")
    boundary = dict(field.split("=", 1) for field in lines[0].split("\t")[1:])
    if (not lines[0].startswith("MOON-BOUNDARY\t") or boundary.get("origin") != "boundary-only"
            or float(boundary["radius_min_km"]) != 200 or float(boundary["radius_end_km"]) != 1700
            or boundary.get("generated_identity") != "none" or boundary.get("min_inclusive") != "true"
            or boundary.get("end_inclusive") != "false"):
        raise ValueError("actual procedural boundary is missing or conflated with a generated identity")
    records, identities = [], set()
    for line in lines[1:-1]:
        if not line.startswith("MOON\t"):
            raise ValueError("unknown catalog record")
        row = dict(field.split("=", 1) for field in line.split("\t")[1:])
        if row["seed"] != "42" or row["planet"] != host or row["system"] != ".".join(host.split(".")[:7]):
            raise ValueError("catalog identity differs from the selected actual host")
        if row["moon"] in identities:
            raise ValueError("duplicate actual moon identity")
        identities.add(row["moon"])
        if not all(math.isfinite(float(row[key])) and float(row[key]) > 0 for key in ("radius_km", "orbit_km", "period_days")):
            raise ValueError("actual catalog properties are missing/nonfinite/nonpositive")
        if row["instance_origin"] == "procedural" and not 200 <= float(row["radius_km"]) < 1700:
            raise ValueError("actual procedural moon radius is outside its retained boundary")
        if row.get("contact") != "unavailable:not-a-contact-observation" or not row.get("property_source"):
            raise ValueError("catalog contact/provenance boundary is absent")
        records.append(row)
    terminal = dict(field.split("=", 1) for field in lines[-1].split()[1:])
    if terminal != {"count": str(len(records)), "planet_limit": "1", "moon_limit": "64000", "scope": "bounded-not-global-minimum"}:
        raise ValueError("actual catalog count does not match its terminal record")
    return boundary, records


def run(args):
    real = player_settings()
    before = fingerprint(real)
    out = outside_real(args.out, real.parent.parent)
    receipt = outside_real(args.build_receipt, real.parent.parent)
    if git("status", "--porcelain", "--untracked-files=no"):
        raise ValueError("catalog attribution requires clean committed source")
    build = verify(receipt, APP, git("rev-parse", "HEAD"))
    out.mkdir(parents=True, exist_ok=False)
    report = {"schema": "moon-fixture-catalog-v1", "complete": False, "build_receipt": build,
              "settings_before": before, "catalogs": [], "fixtures": {}}
    try:
        selected = {}
        report["fixture_sha256"] = {}
        for name in FIXTURES:
            path = ROOT / f"scripts/validation/scenarios/{name}.scenario"
            report["fixture_sha256"][name] = digest(path)
            selected[name] = selections(path.read_text(encoding="utf-8"))
            (out / f"{name}.scenario").write_bytes(path.read_bytes())
        hosts = sorted({row["planet"] for rows in selected.values() for row in rows})
        actual, boundaries = {}, []
        for index, host in enumerate(hosts):
            argv = [str(APP), "--list-moons", "42", host, "1", "64000"]
            result = subprocess.run(argv, cwd=ROOT, capture_output=True, check=False, timeout=90)
            (out / f"host-{index}.stdout.txt").write_bytes(result.stdout)
            (out / f"host-{index}.stderr.txt").write_bytes(result.stderr)
            if result.returncode:
                raise ValueError(f"actual catalog failed for {host}")
            boundary, records = parse_catalog(result.stdout.decode("utf-8"), host)
            boundaries.append(boundary)
            actual[host] = {row["moon"]: row for row in records}
            report["catalogs"].append({"argv": argv, "records": len(records)})
        for name, rows in selected.items():
            report["fixtures"][name] = [{**row, "actual": actual[row["planet"]][row["moon"]]} for row in rows]
        repeated = subprocess.run([str(APP), "--list-moons", "42", hosts[0], "1", "64000"], cwd=ROOT, capture_output=True, check=False, timeout=90)
        (out / "repeat.stdout.txt").write_bytes(repeated.stdout)
        (out / "repeat.stderr.txt").write_bytes(repeated.stderr)
        report["repeat_identical"] = repeated.returncode == 0 and repeated.stdout == (out / "host-0.stdout.txt").read_bytes()
        multi = report["fixtures"]["input-profile-moons"]
        identities = {(row["planet"], row["moon"]) for row in multi}
        systems = {row["actual"]["system"] for row in multi}
        types = sorted({row["actual"]["type"] for rows in report["fixtures"].values() for row in rows})
        count = {host: len(records) for host, records in actual.items()}
        small = [float(row["actual"]["radius_km"]) for row in report["fixtures"]["moon-variety"] if 200 <= float(row["actual"]["radius_km"]) < 201]
        report["coverage"] = {"multi_moons": len(identities), "multi_systems": len(systems), "actual_types": types,
                              "small_sample_radii_km": small,
                              "max_actual_moons_on_selected_host": max(count.values()), "boundary": boundaries[0]}
        if (len(identities) < 20 or len(systems) < 5 or set(types) != {"barren", "ice", "lava"}
                or len(small) < 2 or max(count.values()) < 15 or not report["repeat_identical"]):
            raise ValueError("fixed fixture set did not demonstrate its declared actual catalog coverage")
        verify(receipt, APP, git("rev-parse", "HEAD"))
        report["complete"] = True
    finally:
        report["settings_after"] = fingerprint(real)
        report["settings_unchanged"] = before == report["settings_after"]
        report["complete"] &= report["settings_unchanged"]
        report["limitations"] = ["finite selected set, not the universe's smallest moon", "catalog/approach preparation is not ground/contact/travel proof", "home supplied properties, synthetic home terrain and procedural instance/terrain sources remain distinct"]
        write_json(out / "catalog.json", report)
    print(json.dumps({"complete": report["complete"], "coverage": report["coverage"], "out": str(out)}))
    return 0 if report["complete"] else 1


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build-receipt", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    try:
        sys.exit(run(parser.parse_args()))
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as exc:
        print(json.dumps({"complete": False, "reason": str(exc)}))
        sys.exit(1)
