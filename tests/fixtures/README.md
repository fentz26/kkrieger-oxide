# Original-code regression outputs

These are offline outputs of the pinned Farbrausch reference algorithms, not
application assets. They inherit the Farbrausch BSD attribution in
`../../third_party/farbrausch-LICENSE.txt` and are excluded from the game binary.

| Fixture | Reference | Regeneration |
| --- | --- | --- |
| `animation3383.bin` | Original C++ VM and spline evaluator, every animated development operator at five times | `python3 tools/verify_animation.py` |
| `bitmap-inner.bin` | Original MMX, all 20 pixel modes, 15-bit and full-word inputs | `.reference/venv/bin/python tools/verify_bitmap.py` |
| `bitmap-bilinear.bin` | Original MMX bilinear filter, 1,000 wrapping/clamping cases | Same command |
| `bitmap-fade.bin` | Original MMX Fade64, 512 cases | Same command |
| `noise.bin` | Original scalar noise and RNG initialization, 1,000 coordinate/seed/mask cases | `python3 tools/verify_noise.py` |
| `generators3383.bin` | 966 original non-font generator coefficient sets, reduced-dimension images | `python3 tools/verify_generators.py` |
| `texture-graph3383.bin` | 102 original full-resolution non-font composed outputs, hashes and format/mip fields | `python3 tools/verify_texture_graph.py` |
| `mesh-ring.bin` | 256 original rings and repeated extrusions, with creases and all selection modes | `python3 tools/verify_mesh.py` |
| `mesh-cube.bin` | 128 original cubes: tessellation, UVs, SRT, collision corners and part IDs | `python3 tools/verify_cube.py` |
| `mesh-cube3383.bin` | All 224 development cube parameter sets | `python3 tools/verify_cube.py --development` |
| `mesh-cube-normals.bin` | 128 cubes with original selected normals/tangents and smoothing modes | `python3 tools/verify_cube.py --normals` |
| `mesh-cylinder.bin` | 192 original cylinders, arcs, caps, radial rings and UV seams | `python3 tools/verify_cylinder.py` |
| `mesh-cylinder3383.bin` | All 41 development cylinder parameter sets | `python3 tools/verify_cylinder.py --development` |
| `mesh-subdivision.bin` | 48 selected-face subdivision cases: creases, boundaries and repeated passes | `python3 tools/verify_subdivision.py` |
| `mesh-assembly.bin` | 48 original Add/CollisionCube cases: materials, events, lights, parts and reindexing | `python3 tools/verify_assembly.py` |
| `mesh-graph3383.wz` | 180 composed development geometry outputs through 2,494 original operators, including extrusion, UVs and vertex lighting | `python3 tools/verify_mesh_graph.py` |
| `mesh-deform.bin` | 64 original bending/bevel/displacement pipelines, with MMX-gated bilinear sampling | `python3 tools/verify_deform.py` |
| `mesh-perlin3.bin` | 2,048 original x87 mesh-noise vertices, directed single precision and control-word restoration | `.reference/venv/bin/python tools/verify_perlin3.py` |
| `mesh-multiply.bin` | 64 original multiplication cases: random SRT, local rotation, UV offsets and normal extrusion | `python3 tools/verify_mesh_graph.py --multiply-cases` |

The test tools extract the original functions from the pinned source under
`.reference/fr_public/`; fetch it with `python3 tools/fetch_reference.py`.
The MMX tool compiles an x86 COFF object and executes only the self-contained
function under bounded Unicorn emulation. This permits independent checks on
Apple Silicon without introducing original assembly into the Rust runtime.

`manifest.json` records byte lengths and SHA-256 digests. Regenerate outputs only
from the pinned upstream code; do not substitute Rust outputs for the oracle.

The `.wz` geometry fixture is lossless word-aligned LZ data (`WZ01`, decoded byte
length, then literal or backward-copy word runs). `tools/fixture_codec.py` checks
round-trip equality before writing; the Rust test decoder bounds every run.
The manifest also records the decoded reference length and SHA-256. This codec
is confined to offline fixtures and tests.

For Grid's legacy adjacent-member vector indexing, the native oracle uses an
explicit component accessor. `python3 tools/verify_mesh_graph.py --grid-layout-check`
compares every serialized Grid field between legacy `-O0` and explicit `-O2` builds
and records the matching SHA-256 under `.reference/`.

Material fixtures use the original Init/Exec functions with GPU allocation and
compilation stubbed. They verify CPU state, texture binding, pass ordering and
shared inherited shader updates; they do not verify shader compilation or pixels.
Regenerate with `python3 tools/verify_material.py` and
`python3 tools/verify_material.py --development`; `--beta` stays reference-only.
