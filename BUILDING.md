# Building and checking

## Install

macOS and Linux:

```sh
curl -fsSL https://github.com/fentz26/kkrieger-oxide/releases/latest/download/install.sh | sh
```

Windows (PowerShell):

```powershell
irm https://github.com/fentz26/kkrieger-oxide/releases/latest/download/install.ps1 | iex
```

The installer downloads one compressed executable for the current OS and CPU
(xz on macOS and Linux, gzip on Linux without xz and on Windows), checks it
against `SHA256SUMS` and installs it per user: `~/.local/share/kkrieger-oxide`
with a `~/.local/bin/kkrieger-oxide` launcher, or
`%LOCALAPPDATA%\Programs\kkrieger-oxide` with a Start menu entry. On Linux it
links the installed Arial and Times New Roman fonts for the game and offers to
install missing runtime libraries or the Microsoft core fonts package (which
shows its EULA). Release executables are built with Rust 1.100, std rebuilt
with immediate-abort panics, `opt-level = "z"` and fat LTO. The Release
workflow builds them, installs and runs them on each OS, and publishes them for
`v*` tags.

## Build

Install the Rust toolchain from `rust-toolchain.toml` (1.100.0), then run:

```sh
cargo build --locked --release
cargo run --locked --release
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --tests --no-fail-fast
```

The default build embeds the development project. Released-beta content and
full original-game fidelity remain open; see [PORT_STATUS.md](PORT_STATUS.md).
The `tiny` profile reduces executable size at a measured preparation-time cost.

## Fonts

Runtime text generation needs the requested original Arial and Times New Roman
font files. They are external dependencies and are not redistributed here.
The portable rasterizer searches macOS system font directories and
`/usr/share/fonts/truetype/msttcorefonts`. On Linux, install licensed
copies of the Microsoft core fonts, or set `KKRIEGER_FONT_DIR` to a directory
containing `Arial.ttf` (or `arial.ttf`) and `Times New Roman.ttf`, `times.ttf`,
or `Times_New_Roman.ttf`. The variable is also supported on macOS. Windows uses
installed font faces through GDI and rejects unintended font substitution.

Missing fonts stop uncached atlas generation; no substitute is silently used.
Precomputed original font blobs do not require installed fonts. Matching font
names alone does not prove GDI rasterization parity.

The external-font integration test runs explicitly when these fonts are present:

```sh
cargo test --test font_pages authored_menu_font_packs_real_glyphs_with_windows_line_metrics -- --ignored --exact
```

The normal suite tests cached font pages without external font installation.
CI runs formatting, Clippy, tests and release builds on Linux, macOS and Windows;
it additionally invokes the font integration check on macOS and Windows.

## Platform support

macOS has native audio output. Windows has WinMM output with mocked lifecycle
coverage; real Windows audio-device playback remains unverified. Linux uses ALSA through
`libasound.so.2` and the configured `default` playback device. No ALSA development
headers or linker package are needed to compile. Install the ALSA runtime and
configuration data for your distribution, and configure a working default device.
`KKRIEGER_AUDIO_DEVICE` selects another ALSA PCM name. The `null` PCM discards
samples and is used for headless integration checks:

```sh
KKRIEGER_AUDIO_DEVICE=null cargo test --test audio_output -- --ignored
```

The Linux worker and queue can be checked without a speaker; those checks do not
prove physical-device playback, latency, audible quality or driver parity.
Windowed rendering requires a working graphics environment;
headless unit tests do not establish GPU rendering or complete gameplay parity.
