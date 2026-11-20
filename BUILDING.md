# Building and checking

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
macOS and Windows system font directories and
`/usr/share/fonts/truetype/msttcorefonts` are searched. On Linux, install licensed
copies of the Microsoft core fonts, or set `KKRIEGER_FONT_DIR` to a directory
containing `Arial.ttf` (or `arial.ttf`) and `Times New Roman.ttf`, `times.ttf`,
or `Times_New_Roman.ttf`. The variable is also supported on macOS and Windows.

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
