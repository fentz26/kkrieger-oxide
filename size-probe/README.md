# Size probes

Minimal programs that provide the platform services the game needs on each
OS, with no std and no windowing library: window, input, GPU clear and present,
44.1 kHz stereo PCM16 audio and, where the OS provides it, system font
rasterization. They measure the cost of a native platform layer for a
single-file build of at most 96 KB per OS. `measure.py` reports raw size and
LZMA size, a stand-in for an executable packer. The `ci` feature exits after
120 presented frames; the Size probe workflow builds, measures and runs every
probe on its native runner.

## Budget

| Released beta (`pno0001.exe`, 97,280 bytes, kkrunchy) | Raw | LZMA |
| --- | ---: | ---: |
| Game code `0x801000..0x81f100` in the unpacked image | 123,136 | 68,447 |
| Content, `verified-beta.kx` | 121,767 | 30,800 |

Content is fixed at about 31 KB compressed, which leaves about 65 KB compressed
(about 120 KB of 32-bit x86) for all code, including the platform layer.

## Platform layer

| Probe | APIs | Raw | LZMA | Runner result |
| --- | --- | ---: | ---: | --- |
| Windows i686 | Win32, D3D9, WinMM, GDI | 3,072 | 1,099 | D3D9 device, 120 frames |
| Windows x86_64 | Win32, D3D9, WinMM, GDI | 4,096 | 1,331 | D3D9 device, 120 frames |
| Linux x86_64 | Xlib, GLX, ALSA | 6,648 | 2,251 | 120 GLX frames under Xvfb |
| macOS arm64 | AppKit, Metal, AudioQueue, CoreText | 51,136 | 2,830 | 120 Metal frames |

The macOS raw size is mostly 16 KB page padding.

## Packing

- Windows needs a kkrunchy-class packer that emits a valid PE; the loader
  rejects section alignment below 4 KB.
- macOS: a 94-byte `/bin/sh` header and a `compression_tool -a lzma` payload
  (2,998 bytes in total for the macOS probe) unpack the linker-signed program to
  `$TMPDIR` and run it. Gatekeeper handling of a quarantined download is
  untested; files fetched with `curl` carry no quarantine attribute.
- Linux uses the same shell-dropper approach with `xz` or `gzip`.

## Game code

| Build | Raw | Compressed |
| --- | ---: | ---: |
| macOS arm64 release | 1,317,152 | 541,084 (xz) |
| macOS arm64, build-std, immediate-abort panics, opt-level z, fat LTO | 821,744 | 408,588 (xz) |
| i686 Linux, same settings, gc-linked, without `.eh_frame` | 1,159,644 | 458,088 (xz, x86 filter) |

Stronger general-purpose compressors (zstd 22, brotli 11) do not beat xz on
this code. Context-mixing packers typically gain 10-20% over LZMA, which does
not close the gap: the game code is about 7x the original's code. Non-inlined
x87 rounding helpers leave the size unchanged; the excess is spread across
scene execution, game simulation, mesh and texture generation, GPU paths,
dual development/beta support and std.

## Requirements for a 96 KB player build

1. Compile-time content selection: beta only, without development or
   published-source variants.
2. no_std with alloc; errors abort instead of carrying strings.
3. These native platform modules instead of miniquad.
4. Removal of code the shipped content never reaches, as the original did with
   coverage-guided pruning.
5. Size-oriented code: no generic duplication or closure-heavy paths, and native
   x87 instructions on Windows x86.
6. Fonts from GDI on Windows and CoreText on macOS. Linux has no Arial by
   default and needs FreeType with an installed font.
