# kkrieger-oxide

The goal is a complete Rust rewrite of .kkrieger, preserving the game's content
and behavior without adding new game content. This work is in progress.

The currently runnable window is an early procedural FPS experiment. The first
milestone is a walkable reactor chamber: generated meshes, four generated
materials, point lighting, fog, collision, a small HUD, and synthesized audio.
There are no texture, mesh, font, or sound asset files to ship.

The runnable prototype is **not the complete game** or a faithful recreation
of the 2004 renderer. The original game's levels, enemies, weapons, and V2 music
system have not yet been integrated. The exported operator graph now decodes in
Rust; animation, bitmap generation and all beta mesh classes have been ported and
checked against the upstream implementation. Firing in the prototype currently triggers a
visual flash and synthesized sound; there are no targets or damage mechanics.

## Run

Requires Rust 1.88+ and native graphics support:

```sh
cargo run --release
```

Tested on Apple Silicon macOS using Miniquad's OpenGL backend. The gameplay,
generators, and synthesis are Rust; shaders use GLSL. The platform layer calls
system graphics/window APIs. Audio output currently uses a small macOS
AudioToolbox binding; other platforms run silently until another audio output
backend is implemented. Linux and Windows have not been tested.

| Control | Action |
| --- | --- |
| WASD | Move |
| Left / right arrows | Turn |
| Click | Capture mouse; subsequent clicks fire |
| Mouse while captured | Look |
| Shift | Sprint |
| Escape | Release mouse and clear held keys |
| M | Toggle audio mute |
| R | Reset camera |
| Q | Quit |

Package a standalone macOS application:

```sh
python3 tools/package_macos.py
```

Open `artifacts/Kkrieger Oxide.app`. The app contains the executable and bundle
metadata. It is unsigned for distribution and links to macOS system frameworks;
no external asset directory is required. Pass `tiny` or `speed` to package a
different build profile.

## Size and performance

```sh
cargo run --release -- --bench
python3 tools/measure.py
```

The benchmark runs without a window or audio device. It reports executable size,
mesh and atlas generation, and rendering one second of stereo audio into memory.
It excludes driver startup, GPU rendering, and total process memory. The measure
script saves results for all three profiles to `artifacts/baseline.json`.

Initial Apple Silicon macOS measurements, Rust 1.98.0:

| Profile | Optimization | Executable bytes | KiB |
| --- | --- | ---: | ---: |
| `release` | `s` | 505,120 | 493.3 |
| `tiny` | `z` | 472,592 | 461.5 |
| `speed` | `3` | 554,624 | 541.6 |

These are uncompressed Mach-O executables, including the platform layer and
benchmark mode. They are not comparable to the packed original Windows
executable on an equal basis. Sizes vary with target, compiler, and changes.
96 KiB remains an experimental target, not an achieved budget.

The room generates 1,968 vertices and 984 triangles, occupying 76,752 bytes of
mesh data. Its 512 x 128 RGBA atlas occupies 262,144 bytes after generation.
Early headless spot checks put mesh generation in tens of microseconds, atlas
generation below a millisecond on an otherwise idle machine, and a second of
audio synthesis at roughly 1–2 ms. Re-run locally; these are single-run baseline
observations, not statistical performance claims. The live HUD reports smoothed
frame rate rather than GPU timing.

All release profiles enable LTO, one codegen unit, symbol stripping, and aborting
panics. The default retains size-oriented optimization with vectorization;
`tiny` measures the tradeoff of disabling loop vectorization. Cargo notes that
size profiles are not guaranteed to produce smaller binaries:
[Cargo profile documentation](https://doc.rust-lang.org/stable/cargo/reference/profiles.html).

## Source map

| File | Responsibility |
| --- | --- |
| `src/procedural.rs` | Level recipe, mesh construction, collision, seeded atlas |
| `src/math.rs` | Camera and projection math |
| `src/render.rs` | GPU upload, lighting shaders, input and frame loop |
| `src/hud.rs` | Asset-free rectangles and bitmap lettering |
| `src/synth.rs` | Deterministic synth and macOS audio output |
| `src/main.rs` | Entry point and headless baseline |
| `src/kx.rs` | Original export decoding and dependency ordering |
| `src/animation.rs` | Original animation bytecode and spline evaluation |
| `src/random.rs` | Original wrapping RNG and Perlin noise |
| `src/bitmap.rs` | Original texture arithmetic and generator operators |
| `src/mesh.rs` | Original topology, selections, primitives, normals and subdivision |
| `src/mesh_graph.rs` | Original geometry DAG evaluation, shared outputs and metadata |
| `src/texture.rs` | Original texture graph evaluation and intermediate release |
| `src/bin/kx-inspect.rs` | Offline export inventory and reference fixture transport |

Direct crate dependencies are Miniquad and ttf-parser, pinned in Cargo.toml and
Cargo.lock. Original texture text uses named system fonts; its Windows GDI
coverage and metrics still require validation.
Its platform dependencies are separate from the game code. Unsafe code in this
project is confined to the macOS audio boundary, with buffer and lifetime
invariants documented alongside the calls. The callback performs no heap
allocation; game-to-audio signals use atomics. Audio API reference:
[Apple AudioQueueNewOutput](https://developer.apple.com/documentation/audiotoolbox/audioqueuenewoutput(_:_:_:_:_:_:_:)).

## Validate

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
```

Tests cover mesh index validity, walkable spawn and blocked geometry,
deterministic opaque textures, and finite, bounded, chunk-invariant synthesis.
Native smoke checks covered rendering, movement, turning, cursor capture,
reactor collision, reset, and cursor release. Audio initialization succeeded;
sound quality has not been independently recorded or compared with the original.

## Next milestones

1. Connect original materials, scenes,
   effects, portals and rendering to the exported data.
2. Port the original game simulation, combat, enemy behavior, event execution,
   menus and V2 music/effects pipeline.
3. Replace prototype content with the original game and verify a full playthrough
   against the release executable.
4. Measure and optimize the complete runtime's distribution size, startup, frame
   time, memory and synthesis cost.

The full completion requirements and current reference checks are maintained in
[PORT_STATUS.md](PORT_STATUS.md).

Reference code: [Farbrausch's kkrieger branch](https://github.com/farbrausch/fr_public/tree/master/werkkzeug3_kkrieger).
Its maintainers identify it as a later branch rather than the exact 2004 release
source. The published development export is retained unchanged in `assets/`,
with its license and provenance. Translated original algorithms and reference
test data retain Farbrausch's BSD license; see `third_party/NOTICE.md`.

New project-specific code is MIT licensed; translated upstream code and data
are BSD licensed. Dependency and original-release content licenses still apply.
