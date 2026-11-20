#!/usr/bin/env python3
"""Compress one executable for the installers: xz (LZMA2, preset 9e) or gzip.

Usage: pack.py xz|gz INPUT OUTPUT
"""
import gzip, lzma, pathlib, sys
kind, source, target = sys.argv[1:]
data = pathlib.Path(source).read_bytes()
if kind == 'xz':
    packed = lzma.compress(data, format=lzma.FORMAT_XZ, preset=9 | lzma.PRESET_EXTREME)
else:
    packed = gzip.compress(data, compresslevel=9, mtime=0)
pathlib.Path(target).write_bytes(packed)
print(f'{target}: {len(data)} -> {len(packed)} bytes')
