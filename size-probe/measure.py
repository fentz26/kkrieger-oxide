#!/usr/bin/env python3
"""Print raw and LZMA-compressed sizes, a stand-in for an executable packer."""
import lzma, pathlib, sys
for name in sys.argv[1:]:
    data = pathlib.Path(name).read_bytes()
    packed = lzma.compress(data, format=lzma.FORMAT_ALONE, preset=9 | lzma.PRESET_EXTREME)
    print(f"{name}: {len(data)} bytes raw, {len(packed)} bytes lzma")
