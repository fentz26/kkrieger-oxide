# 96 KB per-OS feasibility probe

Question: can kkrieger-oxide ship as one executable of at most 96 KB on each of
Windows, macOS and Linux? Measured 2026-11-20. "lzma" sizes come from
`measure.py` (LZMA, preset 9e), used as a stand-in for an executable packer.

## Original budget

| Part of the released beta (`pno0001.exe`, 97,280 bytes, kkrunchy) | Raw | lzma |
| --- | ---: | ---: |
| Game code `0x801000..0x81f100` from the unpacked image | 123,136 | 68,447 |
| Content, `verified-beta.kx` | 121,767 | 30,800 |

Content is fixed at about 31 KB. Code therefore has about 65 KB compressed,
about 120 KB raw 32-bit x86, including the platform layer.

## Platform layer (this directory)

Each probe uses no std and calls the OS directly: window, input, GPU
clear/present, 44.1 kHz stereo PCM16 audio and, where the OS provides it,
system font rasterization. CI builds, measures and runs each probe for 120
frames (`--features ci`).

| Probe | APIs | Raw | lzma | CI run |
| --- | --- | ---: | ---: | --- |
| Windows i686 | Win32, D3D9, WinMM, GDI | 3,072 | 1,099 | D3D9 device, 120 frames |
| Windows x86_64 | same | 4,096 | 1,331 | D3D9 device, 120 frames |
| Linux x86_64 | Xlib, GLX, ALSA | 6,648 | 2,251 | 120 GLX frames (Xvfb) |
| macOS arm64 | AppKit, Metal, AudioQueue, CoreText | 51,136 | 2,830 | 120 Metal frames |

The macOS raw size is mostly 16 KB page padding. The platform layer costs
1-3 KB compressed and is not the obstacle. miniquad currently costs about
40 KB raw on macOS.

## Packing

- Windows: a kkrunchy-class packer. Alignment below 4 KB is rejected by the
  loader (`/ALIGN:16` failed with "Exec format error"), so the packer must emit
  its own valid PE.
- macOS: verified locally. A 94-byte `/bin/sh` header plus a
  `compression_tool -a lzma` payload (2,998 bytes total) unpacks the
  linker-signed probe to `$TMPDIR` and runs it to a clean exit. Gatekeeper
  handling of a quarantined download is untested.
- Linux: the same shell-dropper approach applies with `xz`/`gzip`; ELF-native
  unpackers are an alternative.

## Current game

| Build | Raw | lzma/xz |
| --- | ---: | ---: |
| Release, macOS arm64 (CI) | 1,317,152 | 541,084 (xz) |
| Same code, build-std, immediate-abort panics, opt-level z, fat LTO | 821,744 | 408,588 (xz) |
| Same, x86_64 Linux, gc-linked `.text` only | 839,750 | - |
| Same, i686 Linux, gc-linked `.text` only | 857,114 | - |

Our code is about 7x the original's 123 KB of code on x86 and about 6x over
the compressed code budget. Making the x87-rounding helpers non-inlined left
the size unchanged, so emulated x87 arithmetic is not the main cost. The
excess is spread across scene execution, game simulation, mesh/texture
generation, the GPU paths, dual development/beta support and std.

## Conclusion

The platform layer, packing and content are solved or measured for all three
OSes. The open question is the game code: it has to reach about 120-150 KB
raw (about 65 KB compressed) per OS. That needs a size-first player build,
not a library swap:

1. Compile-time content selection: beta only, removing development and
   published-source variants from the player.
2. no_std + alloc; errors abort instead of carrying strings.
3. Native platform modules replacing miniquad (measured above).
4. Coverage-guided pruning of code the shipped content never reaches (the
   original used "lekktor" for this).
5. Size-oriented code: no generic duplication or closure-heavy hot paths;
   x87 instructions directly on Windows x86.
6. Fonts: GDI on Windows and CoreText on macOS cost nothing. Linux has no Arial
   by default, so it needs fontconfig/FreeType with an installed font; the
   glyphs are not identical unless Arial is installed.

Whether step 5 reaches the budget is not yet proven. The next measurement is
one subsystem rewritten size-first, compared before and after.
