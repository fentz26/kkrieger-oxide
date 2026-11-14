#!/usr/bin/env python3
"""Record target-specific binary sizes and headless generator timings."""
import datetime
import json
import pathlib
import platform
import subprocess

root = pathlib.Path(__file__).resolve().parent.parent
results = {
    "recorded_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
    "platform": platform.platform(),
    "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
    "profiles": {},
}
metadata = json.loads(subprocess.check_output(
    ["cargo", "metadata", "--no-deps", "--format-version=1"], cwd=root))
for profile in ("release", "tiny", "speed"):
    subprocess.run(["cargo", "build", "--locked", "--bin", "kkrieger-oxide", "--profile", profile], cwd=root, check=True)
    binary = pathlib.Path(metadata["target_directory"]) / profile / "kkrieger-oxide"
    results["profiles"][profile] = {
        "bytes": binary.stat().st_size,
        "bench": subprocess.check_output([str(binary), "--bench"], text=True),
    }
output = root / "artifacts" / "baseline.json"
output.parent.mkdir(exist_ok=True)
output.write_text(json.dumps(results, indent=2) + "\n")
print(json.dumps(results, indent=2))
print(f"Saved {output}")
