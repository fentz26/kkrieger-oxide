#!/usr/bin/env python3
"""Build a relocatable macOS app without embedding assets or third-party dylibs."""
import json
import pathlib
import plistlib
import shutil
import subprocess
import sys

root = pathlib.Path(__file__).resolve().parent.parent
profile = sys.argv[1] if len(sys.argv) > 1 else "release"
if profile not in ("release", "speed", "tiny"):
    raise SystemExit("profile must be release, speed, or tiny")
subprocess.run(["cargo", "build", "--locked", "--bin", "kkrieger-oxide", "--profile", profile], cwd=root, check=True)
metadata = json.loads(subprocess.check_output(
    ["cargo", "metadata", "--no-deps", "--format-version=1"], cwd=root))
binary = pathlib.Path(metadata["target_directory"]) / profile / "kkrieger-oxide"
app = root / "artifacts" / "Kkrieger Oxide.app"
contents = app / "Contents"
(contents / "MacOS").mkdir(parents=True, exist_ok=True)
shutil.copy2(binary, contents / "MacOS" / "kkrieger-oxide")
with (contents / "Info.plist").open("wb") as stream:
    plistlib.dump({
        "CFBundleExecutable": "kkrieger-oxide",
        "CFBundleIdentifier": "dev.kkrieger.oxide",
        "CFBundleName": "Kkrieger Oxide",
        "CFBundlePackageType": "APPL",
        "CFBundleShortVersionString": "0.1.0",
        "NSHighResolutionCapable": True,
    }, stream)
resources = contents / "Resources"
resources.mkdir(exist_ok=True)
shutil.copy2(root / "LICENSE", resources / "LICENSE")
shutil.copytree(root / "third_party", resources / "third_party", dirs_exist_ok=True)
print(app)
print(f"Executable: {binary.stat().st_size:,} bytes")
