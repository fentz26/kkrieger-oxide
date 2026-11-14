#!/usr/bin/env python3
"""Fetch pinned porting references; never add C/C++ to the runtime build."""
import hashlib
import json
import pathlib
import urllib.request

root = pathlib.Path(__file__).resolve().parent.parent
revision = "8c8f82c2ae01133370e649bbc05916e7aceecadd"
base = f"https://raw.githubusercontent.com/farbrausch/fr_public/{revision}/"
names = [
    "LICENSE.txt", "kdoc.cpp", "kdoc.hpp", "kkriegergame.cpp", "kkriegergame.hpp",
    "mainplayer.cpp", "genbitmap.cpp", "genbitmap.hpp", "genmesh.cpp", "genmesh.hpp",
    "genscene.cpp", "genscene.hpp", "geneffect.cpp", "geneffect.hpp", "genmaterial.cpp",
    "genmaterial.hpp", "_types.cpp", "_types.hpp", "player_kkrieger/kkrieger_oplist.cpp",
    "player_kkrieger/kkrieger_config.hpp", "player_kkrieger/kkrieger_main.cpp",
    "data/kkrieger3383.kx", "engine.cpp", "engine.hpp", "_start.hpp", "_start.cpp",
    "_startdx.hpp", "materials/material11.cpp", "materials/material11.hpp", "genoverlay.cpp",
    "materials/material11.psh", "materials/material11.vsh",
    "materials/material11ps.hpp", "materials/material11vs.hpp",
    "shadercodegen.cpp", "shadercodegen.hpp",
]
manifest = {"repository": "farbrausch/fr_public", "revision": revision, "files": {}}
for name in names:
    path = "werkkzeug3_kkrieger/" + name
    data = urllib.request.urlopen(base + path, timeout=30).read()
    destination = root / ".reference" / "fr_public" / path
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_bytes(data)
    digest = hashlib.sha256(data).hexdigest()
    if name == "data/kkrieger3383.kx":
        assert digest == "f96af47de9558161b3203240af412c8050a574d2b72ac56323e7bb69554fcac6"
    manifest["files"][path] = {"bytes": len(data), "sha256": digest}
output = root / ".reference" / "source-manifest.json"
output.write_text(json.dumps(manifest, indent=2) + "\n")
print(output)
