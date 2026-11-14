#!/usr/bin/env python3
"""Reproduce local beta reference extraction; no original code runs natively.

The archive stays unchanged under .reference. Its freeware terms and content
are not relicensed by this tool. Unicorn and the pinned extraction helpers are
development-only and are never dependencies of the Rust game.
"""
import hashlib
import io
import json
import pathlib
import subprocess
import sys
import urllib.request
import zipfile

root = pathlib.Path(__file__).resolve().parent.parent
ref = root / ".reference"
ref.mkdir(exist_ok=True)
revision = "3bf0ff017372e640e966c2785a4d95a998cec242"
archive_url = "https://files.scene.org/get/parties/2004/breakpoint04/96kgame/kkrieger-beta.zip"
archive_sha = "0de0b9abafd78cf7f57fb264c7136ace52b2653f5c675574d0f10eda0a654c9f"
exe_sha = "ac2192b740c57125067a29d2e0ea6d95b222543398188c5f67bbf01157622685"
export_sha = "503c9cda010801a5df95928574f2b9abf40f270396444a2ac79a5c5f233bf0c2"
helpers = {
    "kxread.py": "3d3a4d6c114cb059c3828ae18a45358f5099db9e8c9e608b0b4d3c2ddb6fa137",
    "unpack_beta.py": "ae53dc4250047f8e80f65bc3d9fb3c77308e085dde67bc3fb024466740b6d811",
}
def digest(b):
    return hashlib.sha256(b).hexdigest()

def fetch(url, sha):
    b = urllib.request.urlopen(url, timeout=30).read(4 * 1024 * 1024)
    if digest(b) != sha:
        raise SystemExit(f"Reference checksum mismatch: {url}")
    return b

archive = ref / "kkrieger-beta.zip"
if not archive.exists() or digest(archive.read_bytes()) != archive_sha:
    archive.write_bytes(fetch(archive_url, archive_sha))
with zipfile.ZipFile(io.BytesIO(archive.read_bytes())) as bundle:
    exe = bundle.read("pno0001.exe")
    assert digest(exe) == exe_sha
    (ref / "pno0001.exe").write_bytes(exe)
    (ref / "beta-readme.txt").write_bytes(bundle.read("readme.txt"))
directory = ref / "wasm"
directory.mkdir(exist_ok=True)
base = f"https://raw.githubusercontent.com/MasonDye/kkrieger-wasm/{revision}/werkkzeug3_kkrieger/wasm/tools/"
for name, sha in helpers.items():
    b = fetch(base + name, sha)
    if name == "unpack_beta.py":
        text = b.decode()
        original = "mu.emu_start(base + entry, 0)"
        assert text.count(original) == 1
        text = text.replace(original, "mu.emu_start(base + entry, 0, timeout=20_000_000, count=200_000_000)")
        b = text.encode()
    (directory / name).write_bytes(b)
python = ref / "venv" / "bin" / "python"
if not python.exists():
    subprocess.run([sys.executable, "-m", "venv", str(ref / "venv")], check=True)
check = subprocess.run([str(python), "-c", "import unicorn; assert unicorn.__version__ == '2.1.4'"], capture_output=True)
if check.returncode:
    subprocess.run([str(python), "-m", "pip", "install", "unicorn==2.1.4"], check=True)
subprocess.run([str(python), str(directory / "unpack_beta.py"), str(ref / "pno0001.exe"),
                str(ref / "verified-beta.kx"), str(ref / "beta-image.bin")], timeout=30, check=True)
export = (ref / "verified-beta.kx").read_bytes()
if digest(export) != export_sha:
    raise SystemExit("Extracted beta differs from the independently verified export")
manifest = {
    "source": archive_url,
    "extractor": f"MasonDye/kkrieger-wasm@{revision}",
    "helper_sha256": helpers,
    "emulation_limits": {"timeout_microseconds": 20_000_000, "instructions": 200_000_000},
    "image_data_offset": "0x6168c",
    "files": {name: {"bytes": (ref / name).stat().st_size, "sha256": digest((ref / name).read_bytes())}
              for name in ("kkrieger-beta.zip", "pno0001.exe", "verified-beta.kx")},
}
(ref / "beta-provenance.json").write_text(json.dumps(manifest, indent=2) + "\n")
print(f"Verified original export: {len(export)} bytes / {export_sha}")
