# Full-game rewrite

Required outcome: the entire original .kkrieger game implemented in Rust,
preserving its content and gameplay, with no invented game content. Runtime
performance and distribution size must be measured and optimized. The authored
runtime is implemented in Rust, but complete original-game fidelity and a full
playthrough remain unverified. The goal remains active.

## Current runtime evidence

This section supersedes historical “pending” statements in the progress notes
below; those notes record the state when individual subsystems were ported.

- The main executable loads the unchanged development project, executes its
  authored intro, menus and gameplay roots, and uses the shared original-order
  simulation, events, collision world and actor registry. It no longer runs the
  invented prototype room or HUD.
- Native macOS audio plays the Rust V2/Ronan synthesizer and precomputed VFX bank.
  Game sounds and listener updates reach the mixer, with rotating effect slots,
  distance attenuation, stereo positioning and Doppler resampling. Portable
  speaker routing and interpolation are not verified against a Windows driver.
- Music restarts, seeks and effect-request processing have allocation-counting
  checks. Synth reset and MIDI workspace reuse avoid heap operations in those
  tested paths. Audio output uses the original 44,100 Hz stereo PCM16 format.
- The authored menu GPU capture shows readable text after correcting OS/2
  Windows ascent/descent offsets. GDI hinting and exact coverage remain unproven.
  Menu navigation and original resolution/shadow options have regression checks.
- A five-minute simulated movement/firing run passes 30,000 ticks and 60,002
  scene passes, checking actor count, player placement and finite render geometry.
  This is a CPU render-preparation check, not five minutes of GPU pixel parity,
  level traversal, or a complete playthrough.
- A separate 30-tick GPU movement/firing check passes 62 scene passes. The menu
  checker exercises compositing and captures the final target. Shadow disabling
  retains mesh draws and produces no shadow draws in the tested authored frame.
- Build-time project packing decodes byte for byte to the original export. The
  refreshed macOS ARM64 release executable is 1,300,656 bytes. One `--bench` run
  precalculates the embedded development intro assets in 2,634.206 ms, producing
  10 meshes, 138,690 vertices, 11 materials and 22 bitmaps (46,137,344 host pixel
  bytes). This is an asset-preparation measurement, not gameplay frame timing
  or a released-beta content benchmark. Further size optimization remains open.

The refreshed local audit passes 479 tests across 210 test-binary result blocks,
with 32 ignored checks (`cargo test --tests --quiet`, Rust 1.100.0).
`cargo fmt --check` and Clippy across all targets with `-D warnings` pass.
Ignored GPU/native/long-running checks require explicit execution; the count
does not imply their requirements are complete.

Remaining completion gates include released-beta ABI/content differences,
original-executable pixel and full-gameplay comparisons, complete level traversal
and ending, cross-platform audio, and further size/performance measurement.
The runtime currently mixes some verified released light ordering with published
source geometry; that combination must not be presented as released-binary parity.

## Mesh animation follow-up

`prepared_mesh::skinning` ports the later source's `EngMesh::EvalBones` routine.
A pinned C++ oracle checks 2,048 weighted vertices, including normal/tangent
cleanup, bit for bit in debug and release. This routine is not yet wired into
GPU rendering. The development export uses the older GenMesh path; its editor
bone recorder is excluded under `sINTRO`, while the later engine uses GenMinMesh
animation ownership and per-job animation matrices. These paths must be traced
separately before attaching skeleton data or claiming animated-mesh parity.

Explicit release-mode full-asset checks currently pass for development and
locally extracted beta collision worlds and prepared geometry, plus authored
menu/gameplay paint dependencies and gameplay texture/font resources. They
verify preparation and bounds/index invariants, not complete animated rendering.

## Released IPP dispatch follow-up

The pinned released handler table identifies `0xea` at `0x814a0e` and `0xf0`
at `0x814683`. `tools/inspect_beta_ipp.py` verifies the executable hash, table
records and complete seven-instruction Select wrapper. Beta Select calls
`KOp::ExecInputs` without an authored count or output-reference adjustment;
Rust now distinguishes this from the later published Select handler. A regression
checks repeated beta selection with unchanged references. The released one-word
viewport handler is structurally different from the published viewport and still
requires full call/state tracing before runtime support can be claimed.

The released viewport wrapper now has 24 bounded Unicorn traces in
`tools/verify_beta_viewport.py`, with pinned executable and routine hashes.
They cover cache hits/misses, all four size values and signed output counts.
Misses execute input zero before target allocation, prepare a viewport, clear
its final word, invoke the paint callee and assign LastOutput. Hits only find
and assign the cached output. Callees are controlled stubs: camera state and
actual rendering remain unverified, so this evidence alone does not enable
beta viewport support.

## Reference state

- Official source pinned to `farbrausch/fr_public@8c8f82c2ae01133370e649bbc05916e7aceecadd`.
- `assets/kkrieger3383.kx`: unchanged development export, SHA-256 in `assets/README.md`.
- `.reference/verified-beta.kx`: independently extracted from the original
  scene.org release using Unicorn 2.1.4. Extraction stopped before executing
  game API calls, with a 20-second/200-million-instruction bound. The resulting
  121,767 bytes match MasonDye's independently published export byte for byte.
- `.reference/beta-provenance.json` records archive, executable, export hashes
  and extraction offset. Original beta content is not relicensed by this port.
- No C++, original assembly, or emulator is linked into the Rust runtime.
  Original C++ and MMX are used only as offline test oracles.

## Portal execution progress

Rust now queues original portal and sector jobs, projects portal boxes, traverses
cost-limited sector connections and retains bounds across views. All 128 graph
cases at two views match the enabled original C++ visibility branch. Both exports
register all 47 sectors and 35 portals; all 47 sector contents execute through
their currently reached handlers. This is graph execution coverage, not a playable
game or collision-query validation.

Static inspection confirms the released beta contains the corresponding recursive
routine and edge table. Its sector layout and some visibility branches differ
from the published source; this is structural evidence, not beta behavior parity.
`tools/inspect_beta_portals.py` records that distinction and the executable hash.

The Delay envelope state machine matches 20,480 persistent calls to the original
handler. Delay and If now execute in the scene interpreter using 256 game
switches and explicit current/previous time. Sequential instance lists distinguish
repeated operators, retain branch state and restore nested scene cursors. Sector
execution and If children use the original per-child SceneMemLink scope.
Malformed replay reports an error rather than silently reallocating state.
Gameplay must still supply switches and time; full player physics and other
handlers remain pending.

Collision-cell geometry now includes the original face ordering, degenerate
triangle fallback, warped-face splitting, double-precision normal normalization,
subtractive overlap and bounding boxes. The 256-case original C++ fixture checks
plane counts, plane coefficients, center, bounds and mode fields. Dynamic cell
preparation, transforms and the original orthonormalized matrix interpolation
match another 128 C++ cases bit for bit. Physic now registers persistent cells
through the sequential instance lists, executes original inputs, and exposes the
original dynamic-cell physics phase and blocked endpoint updates. Non-cell
particle/box branches are commented out in the published source and stay inactive.
Cell point masks, plane rejection, approximate distance and inside/outside ray
intersection queries are ported, with a 4,096-call original C++ regression fixture.
The source negative-distance convention and parallel divisions are preserved.
Collision-cell connectivity now preserves the original heap sort and ordered
add/sub/zone adjacency; 128 original C++ graphs cover empty lists, equal keys
and all cell modes. Collision-face generation now ports the original polygon
clipper, recursive splitting and linked-list ordering, with 128 complete native
reference cases including warped cells with 12 planes. The source extra-plane
read is represented explicitly: a zero unused slot below 12 planes, or Center
immediately after the original 12-plane array. Static-world assembly now follows
SetSceneR/AddMesh: repeated transforms, child-before-parent cell order, root and
sector identities, collision logic and event spawn records. Its C++ fixture covers
64 scene traversals. World::from_document now initializes sectors and batches
meshes from the same graph pass, then builds connectivity and collision faces.
Both complete static level graphs assemble in release mode:

| Export / scene root | Cells | Adds | Subs | Zones | Faces | Event records |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Beta / 4357 | 7,745 | 1,858 | 5,737 | 150 | 50,285 | 256 |
| Development / 5171 | 6,767 | 1,858 | 4,748 | 161 | 50,247 | 260 |

Both original player start positions resolve to additive cells in registered
sectors; the initialized graphs retain all 47 sector identities. These are Rust
integration results, not full original-executable world parity. Connected-cell
ray traversal now follows the original 16-cell limit, checks static/dynamic
obstacles and accumulates zone entry/exit/contact masks. All result fields and
dynamic-cell identity match 1,280 original C++ queries. Source selective field
copying at outer walls is preserved. Traversal uses a fixed stack buffer.
Sphere/edge and sphere/face checks and per-face soft response are ported,
including false-result nearest-point writes and coincident-contact arithmetic.
The native regression fixture covers 4,096 cases. Complete MoveCollider now
traverses generated faces and dynamic cells with the original player/monster
exclusions, updates zones and active cells, and retains the original return
value and radius semantics. Another 1,280 complete C++ calls check movement
outputs and zone masks. Integration checks run 64 small movement steps from
each original level start and verify finite positions within additive cells.
These are static-world checks; dynamic level execution, event scheduling, player
simulation and the game loop remain pending.

## Implemented and checked

- Bounds-checked Rust decoding of the original beta, development export, and
  newer header format. Includes compact floats/integers, graph connections,
  raw parameter records, strings, animation, events, splines, and blobs.
- Dependency reachability and initialization ordering, including event link
  semantics and cycle detection. Malformed and truncated input checks.
- Both complete input graphs parse: beta 4,816 operators / 77 classes / 18
  events / 38 splines; development 5,174 / 78 / 2 / 23. All bytes consumed.
- Original vector animation VM, masked stores, scoped variable restoration,
  spline evaluation, and wrapping RNG in Rust. All 113 development and 149 beta
  animations compared with the original C++ implementation at five times.
- All 20 original integer texture inner modes match the original MMX code
  bit for bit over 1,280 four-channel pixels, including full 16-bit edge cases.
- Packed color interpolation matches MMX for 512 cases, including out-of-range
  fade values. Perlin noise matches original scalar C++ bits for 1,000 signed,
  wrapped coordinate/seed/mask combinations.
- Texture graph evaluator releases intermediates after their final consumer
  and uses copy-on-write when a texture branches. Unsupported operators are
  errors; they never become invented fallback assets.
- All bitmap classes used by the original beta graph are implemented: Flat,
  Perlin, Color, Merge, Format, Dots, Mask, HSCB, Gradient, Range, GlowRect,
  Blur, Rotate, Distort, Normals, Bump, Text, Cell, RotateMul, Twirl, Sharpen.
- Bilinear sampling matches original MMX in 1,000 cases. Original C++ generator
  comparisons pass for 966 development and 780 beta coefficient sets on small
  test images. All 102 development and 95 beta non-font texture outputs match
  C++ composed graphs at full resolution, including format and mip settings.
- Batched texture evaluation shares dependencies and retains requested roots.
  All 115 beta texture outputs generate at full resolution (238,026,752 output
  bytes; one local release run took 4.8 seconds). This includes text rendered
  from the original named system fonts using Rust outline rasterization.
- Text supports original range grammar, atlas packing, cached coverage, CP1252,
  and font-page metrics. Windows GDI coverage and metric parity remain unverified;
  font-dependent graph outputs are explicitly excluded from the C++ comparisons.
  Original Arial and Times New Roman fonts must be installed or supplied using
  `KKRIEGER_FONT_DIR`; missing named fonts produce errors.
- Original mesh element fields, selection operations, half-edge traversal and
  two-sided Ring initialization, face splits, bridges, vertex-cycle repair and
  extrusion are ported. A native oracle executes unchanged original C++ for
  256 rings and repeated extrusions, including creases and all selection modes:
  topology, masks, skin weights and attribute storage match (trigonometric
  positions use a 2e-6 tolerance).

- Original Cube includes axis tessellation, all UV options, crease separation,
  SRT, origin offset, collision-cell corners and convex part IDs. All 224 cubes
  in each original export match the native C++ oracle, plus 128 synthetic cases.
- Original Cylinder includes arcs, cap triangulation, radial rings, UV seams,
  cap deletion and origin offset. All 41 development and 39 beta cylinders
  match original C++, plus 192 synthetic cases.
- Original normals and tangents match 128 C++ cases with mixed selections,
  crease/smooth modes and component masks. Selected-face subdivision, attribute
  creases, boundaries and reindexing match 48 C++ cases with repeated passes
  and alpha values from zero through 1.5.
- Mesh checks compare all topology and element fields, vertex attributes,
  weights, collision data and part IDs; floating attributes use a 2e-6 tolerance.
  C++ cube oracle adapts only x86 stack argument indexing to an explicit array.
  These checks establish source-algorithm parity, not full game/playthrough parity.

- Mesh assembly preserves topology, material/pass deduplication, collision events,
  light-slot and part reindexing, and pivot reset. General CollisionCube supports
  rectangular subdivisions and radial wedges. Forty-eight C++ assembly cases
  compare all geometry and metadata, including byte-truncated event fields.
- Geometry DAG evaluation shares outputs and releases intermediates after their
  final consumer. Supported original operators include primitive creation,
  selections, subdivision, SRT/attribute transforms, triangulation, addition,
  face deletion, material linking, collision cells, bending, bevels, shadow flags
  and SingleVert.
- Composed geometry matches C++ for 180 development outputs through 2,494 operators
  and 171 beta outputs through all 2,376 beta mesh operators, including full element/attribute
  arrays, materials, event references, normal-cache state, collisions and parts.
  Unsupported or animated geometry branches fail explicitly. Material and event
  handles are retained for their still-unported rendering/execution subsystems.

- SRT-blended bending, curve bending, bevels and shared-vertex texture
  displacement are ported. Sixty-four combined C++ pipelines compare all mesh
  fields and attributes. The displacement oracle gates its scalar bilinear
  adapter against 1,000 emulated original-MMX samples before generating cases.
  Displacement uses typed bitmap inputs in the geometry DAG. Required textures
  generate once per batch, with last-consumer release. Composed geometry checks
  include displacement driven by full-resolution original C++ texture outputs;
  original pixel data stays in reference-only files.

- Mesh vector Perlin preserves original x87 single-precision, downward rounding
  through portable Rust arithmetic, without changing the host FP environment.
  All 2,048 golden vertex results match original x87 bits, including grid
  boundaries, negative coordinates and untouched vertices. The bounded COFF
  oracle executes original control-word and floor-conversion assembly offline.
- The native composed-geometry oracle gates its FP32 directed-rounding adapter
  against those x87 outputs before evaluating graph data. Mesh multiplication
  includes chained/random SRT, local rotation, UV offsets, normal extrusion,
  materials and collision reindexing; 64 additional C++ cases cover all modes
  and zero/negative counts. Centering and automatic collision bounds are ported.
- All mesh classes used by the beta are implemented, including grouped extrusion,
  directional/point/slot/ambient vertex lighting, light slots and cubic,
  cylindrical and spherical UV projection. Full composed beta geometry compares
  every vertex attribute and topology/selection field with the original C++.
  All development mesh classes are implemented, including the eight Grid nodes.
  All 2,531 development geometry nodes execute with the original named fonts;
  37 font-dependent nodes remain outside GDI differential coverage. Material
  handles are preserved; material execution is still pending.
- The development geometry fixture is losslessly word-packed from 129,806,496
  reference bytes to 31,623,612 bytes. Its test-only decoder adds no dependency
  or data to the game. The fixture manifest records both encoded and decoded
  SHA-256 digests; beta golden data remains reference-only.

- Grid covers original two-sided topology, quad splitting, normals, UV mirroring,
  side/edge masks and optional backface deletion. The C++ oracle uses explicit
  vector component access: optimized Clang otherwise drops the legacy z-edge
  test. All 5,200,928 serialized Grid bytes match between legacy/O0 and explicit/O2
  reference builds; the beta geometry reference hash remains unchanged.


- `src/material.rs` now constructs Material11 pass schedules and Material_Add
  graphs from both complete data sets. It preserves source parameters, texture
  links, shared shader identities, pass ordering, finalizer size/aspect and
  insert-pointer behavior. All 48 CPU state words, texture assignments and animated pass updates
  match original C++ across 256 synthetic cases and both complete material
  graphs (padding excluded in graph comparisons). GPU shader translation now compiles (see below);
  rendered parity and full rendering integration remain unverified.

- `src/scene.rs` constructs the original scene cache shapes for every scene
  operator in both exports, retaining mesh/effect handles for preparation.
  `scene::exec` evaluates the original animation VM while issuing transform,
  multiply, camera, light, paint and deferred sector jobs. Unsupported dynamic
  handlers fail explicitly and discard the incomplete job list. Both data sets
  pass initialization and supported-job execution checks; scene CPU differential
  fixtures, mesh preparation, dynamic execution and GPU integration remain open.

- `src/prepared_mesh.rs` ports the CPU EngMesh buffers: packed color and
  tangent-space vertices, part/material bounds, per-program job reuse, triangle
  indices, shadow planes/silhouettes, sprite points and thick-line endpoints.
  All fields match 512 original C++ preparation jobs (float tolerance 2e-6).
  The batch path resolves original material programs and retains their bindings.
  GPU upload, shader compilation and actual drawing are still pending.

- `material::shader` translates the original Material11 vertex/pixel programs
  to GLSL, including texture/environment transforms, combiners, bump lighting,
  specular approximation, alpha test, fog and shadow extrusion. All 248 distinct
  development and 218 beta pass programs compile on the local OpenGL driver via
  `cargo run --bin shader-check` (add `-- --beta` for the release data).
  `material::pipeline` maps source depth, blend, stencil and culling settings.
  Lighting lookup generators match every original C++ texel byte. Full draw
  integration and original-vs-Rust pixel comparisons
  remain open; successful compilation is not visual fidelity evidence.

- Material constant setup now matches 128 original C++ Set calls, preserving
  original transpose-based model inverse, light attenuation, texture SRTs,
  environment/projection transforms, fog and specular constants. Unwritten
  source registers are zero-filled and excluded from comparisons.
- `texture_upload` converts player RGBA and signed normal maps and generates
  original 15-bit mips. All output bytes and mip limits match 256 native C++
  UpdateTexture cases, including rectangular, NPOT and inverted-filter inputs.
  The original unwritten tails of rectangular mip allocations remain explicit.
  The OpenGL upload path now reads back all written mip levels correctly.

- `gpu_texture` uploads unsigned and signed source bytes without converting
  signed maps through unsigned interpolation. Source filtering, wrapping,
  anisotropy and mip allocation limits are retained. On the local OpenGL driver,
  32 unsigned/signed sampler cases read back every written mip correctly.
- `gpu_mesh` uploads original static job vertex/index buffers and draws with
  the translated shaders, material constants and render states. An offscreen
  original Cube draw verifies packed D3DCOLOR decoding by pixel readback.
  These are GPU integration checks, not an original-executable image comparison.
  Dynamic finalizers, complete scene execution, shadows and full-game rendering
  still require integration. The playable executable remains the old prototype.

- `shadow` matches 128 original C++ shadow volumes, including homogeneous
  vertices, silhouette order, cap indices and retained face-classification state.
  The GPU path now streams these indices into original two-sided stencil passes
  on D24S8 targets. Z-pass and z-fail produce identical nonempty 64² stencil masks
  in the offscreen integration test while leaving depth unchanged.
- Miniquad 0.4.11 enables depth testing only when depth_write is true and does not
  apply GL depth write masks. The renderer explicitly applies original read-only
  depth tests and polygon bias. Clear wrappers temporarily open write masks and
  restore them; scissor continues to clip Clear as specified by Direct3D 9.
  Pixel readback checks color, depth and stencil clears under closed write masks.
  Full-scene light/shadow scheduling and original-executable pixel parity remain
  unverified; sprites, gameplay, audio and the playable runtime are still pending.

## Remaining requirements

1. Validate original text/font behavior against Windows GDI, including original
   font-page use by scene rendering. Verify source-branch differences against
   the original beta executable.
2. Integrate mesh preparation with the material/scene runtime. Validate original
   release/source-branch differences and font-dependent development geometry.
3. Port material/render passes, scene transforms and instances, lights,
   sectors, portals, camera, skinning/walking, particles and postprocessing.
4. Port the original animation/event-driven execution and per-instance state
   around the implemented VM. Preserve timing and the original menus/credits.
5. Port all game simulation: player motion, collision/physics, weapons,
   projectiles, enemy types/AI, triggers, switches, pickups, doors, damage,
   death/restart, level progression and ending.
6. Port the V2 synthesizer/player and original song/effects pipeline; replace
   temporary prototype audio. Preserve audio output on target platforms.
7. Replace the invented prototype room, HUD and emitter with original content.
   Connect the whole original graph to the runtime. No unsupported operator
   may be silently skipped in a completed build.
8. Verify a complete playthrough against the original executable, including
   controls, collision, rendering, menus, combat, audio and completion states.
9. Measure release size, startup, frame time, memory and synthesis cost on
   supported targets. Optimize demonstrated hot paths and operator reachability.

## Checks and reference regeneration

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
cargo test --test kx_data --test animation_reference -- --include-ignored
cargo run --bin kx-inspect -- assets/kkrieger3383.kx
cargo run --bin kx-inspect -- .reference/verified-beta.kx --beta

cargo run --release --bin kx-inspect -- .reference/verified-beta.kx --beta --world=4357
cargo run --release --bin kx-inspect -- assets/kkrieger3383.kx --world=5171
cargo test --release --test collision_world -- --include-ignored

python3 tools/fetch_reference.py
python3 tools/prepare_beta_reference.py
python3 tools/verify_animation.py
python3 tools/verify_animation.py .reference/verified-beta.kx --beta
python3 tools/verify_noise.py
python3 tools/verify_generators.py
python3 tools/verify_generators.py --beta
python3 tools/verify_texture_graph.py
python3 tools/verify_texture_graph.py --beta
python3 tools/verify_texture_graph.py --displacement-inputs
python3 tools/verify_texture_graph.py --beta --displacement-inputs
python3 tools/verify_mesh.py
python3 tools/verify_cube.py
python3 tools/verify_cube.py --development
python3 tools/verify_cube.py --beta
python3 tools/verify_cube.py --normals
python3 tools/verify_cylinder.py
python3 tools/verify_cylinder.py --development
python3 tools/verify_cylinder.py --beta
python3 tools/verify_subdivision.py
python3 tools/verify_assembly.py
python3 tools/verify_deform.py
python3 tools/verify_mesh_graph.py
python3 tools/verify_mesh_graph.py --beta
python3 tools/verify_mesh_graph.py --multiply-cases
python3 tools/verify_mesh_graph.py --grid-layout-check
.reference/venv/bin/python tools/verify_perlin3.py
cargo test --test mesh_reference -- --include-ignored
cargo test --release --test texture_graph_reference -- --ignored
.reference/venv/bin/python tools/verify_bitmap.py
```

The ignored beta checks require the local extraction and its original C++
oracle output. Oracle-generation tools require Clang; the MMX oracle additionally
requires Unicorn in the reference-only Python environment. Generated regression
fixtures are not shipped in the game executable. Existing prototype binary
measurements do not measure the unfinished full-game runtime.

## Gameplay operator state

The original parameter and weapon-event operators now execute in the scene
interpreter. Inventory and ownership reset only when ResetByOp is set; maxima,
movement/camera tuning and weapon hit/cooldown/flag arrays update every call.
Weapon event groups preserve null links and register pointers without calling
event links marked OPC_DONTCALLLINK. Native C++ fixtures cover 384 repeated
parameter applications and 384 event registrations. Complete player simulation,
combat, event scheduling and runtime integration remain pending.

The original player movement prefix is now ported: ground probing, damping,
acceleration, gravity, integer stair lift, movement history, landing sound requests
and hit-cell transforms. A 768-slice C++ fixture checks every output bit. Player
orientation evaluates trig at higher precision before storing floats to avoid an
observed single-precision host-library rounding difference. This does not yet
implement the remaining OnTick phases or audio playback. Both authored levels
also pass 64 movement-prefix steps with hit-cell physics updates between steps.

Jump input now preserves the original ground/timer gate, impulse, ten-tick timer
and sound request. Player damage preserves armor-dependent integer damage, life
clamping, unchanged armor and hurt sound conditions. A native 1,024-action fixture
also checks original sound-helper volume and distance ranges. Sound playback and
the remaining player/input/game-tick phases are still pending.

Weapon animation, footstep timing, switch decisions and firing eligibility now
match 1,024 native C++ frames. The port preserves free weapon-zero firing,
continuous-fire flags, null event links, dry-fire sounds, ammo consumption and
cooldown rounding. Returned FireShot requests and weapon-event reset decisions
still need projectile creation and event lifecycle consumers. Splash processing
remains a separate phase between footsteps and firing, as in the source.

The eight original ShotInfo definitions and FireShot initialization are ported.
Player muzzle transforms, monster direction transforms, identity shot bases,
old-position velocity encoding and rejection-sampled cube tensors match 128
native C++ initializations bit for bit, including subsequent RNG state. Spawn
event metadata is retained; persistent event allocation, shot-list integration,
projectile updates, collision damage and explosion processing remain pending.

ShotAI movement, cube damping/rotation/gravity, collision response and ordered
damage/explosion requests now match 64 native C++ trajectories bit for bit, with
explicit coverage for both player and monster damage. The original bounce-state
reset is preserved. Consumers must apply damage before queuing the corresponding
event. Persistent shot/event lifecycle and complete game-tick integration remain
pending. The repository is now pinned to Rust 1.100.0.

Monster damage now updates health and activation state, with original dead-monster
guards and byte-sized kill-switch increments. A native 1,024-call fixture covers
armor, zero/negative switch disabling and overflow. Projectile outcomes now have
a state consumer that applies player/monster damage before returning explosion
metadata for event queuing. Monster registration/AI and the event scheduler remain
pending.

SplashCalc and the complete splash-damage phase now match 64 native C++ cases,
including visibility rays, outside-volume bypass, monster activation on zero
damage, player hit halving and kill switches. The splash scene operator (0x14)
uses per-instance rising-edge state and world-space placement; another 512 native
executions check triggering and retained pending payloads. Scheduling this phase
within the complete game tick and consuming dynamic events remain pending.

Checkpoint registration (scene operator 0x13), Continue and RespawnAt now retain
original placement order and state changes. A 64-case native C++ fixture checks
collision fallback, requested camera coordinates, hit-cell matrices, movement
timer preservation, living-monster reset and minimum ammo. Scene execution keeps
authored checkpoint coordinates independent of the parent transform. The player
hit-cell handle remains stable across placement. Input mouse-counter refresh,
Restart zone/event cleanup and complete tick integration remain pending.

The original Zones phase now processes authored trigger/action codes in world
zone order, including conditional switch mutations, bounded inventory changes,
weapon pickup selection, jump impulses and checkpoint activation. Respawn timers
and static/dynamic collision interaction cleanup match 3,328 native C++ phases.
Expired timers return ordered world-entry reset requests; native chain deletion
is checked through a test consumer. The persistent event scheduler must consume
these requests when integrated into the full simulation tick.

KEvent runtime values, timeline import and KEnvironment event registration/expiry
now have a Rust scheduler. Native fixtures check 768 schedules and 128 scoped
contexts, including equal-time activation, newest-first lists, swap removal,
interval/variable mapping, inherited splines and Stop. Scene Time/Event operators
(0x05/0x06) execute against isolated event instance scopes, restore caller context,
apply event matrices before parent transforms and initialize durations. Explicit
begin_events/finish_events bracket scene and deferred-sector execution. Event
instance reset/expiry releases its scope; gameplay shot/zone/weapon registration,
trigger creation and monster-owned child-chain cleanup still need integration.

Misc_Trigger (0x07) now creates dynamic scheduler events from per-instance state.
Its 2,048-call native fixture covers first-use suppression, threshold inversion,
repeating timers, parent-position payloads, invalid event links and float-to-int
timing behavior. The source's unused color/spline arguments remain unused. Scene
integration checks registration on the next frame and independent event memory
across repeated emissions. Full game-event registration and tick scheduling are
still pending.

Game AddEvents now registers persistent shot, zone and weapon events with the
scene scheduler. Its 384-phase native fixture checks original registration and
swap-removal order, respawn selection, optics switching and weapon event fields.
Integration tests check zone/weapon instance resets and projectile event release
with explosion registration on the next frame. FireShot and the ShotAI ownership
loop now create/consume actual scheduler events and apply damage before queuing
explosions. The source Select removal path preserves its distinct no-Exit behavior.
Complete tick/input/monster integration remains pending.

Restart now resets all monster states, all zone event-instance chains and
switches 16..255 while preserving options, inventory and movement timers until
the operator-reset phase. Unlike Continue, its camera starts at the resolved
collision position. Native fixtures check 64 restarts and 256 per-slice camera
updates, including height damping, normal and zoom-position camera spaces.
The runtime restart consumer also clears persistent zone scene memory. Mouse
counter refresh and placement within the complete tick/input loop remain pending.

Player-build OnKey handling now matches 3,072 native calls, including release
behavior, weapon mapping, use/fire/jump, hurt sounds, fly mode and original cheat
sequences. Mouse look matches 1,024 original prefix executions, including speed,
inversion, angle clamping and the one-second pause cutoff. Respawn keys refresh
mouse counters; a placement refresh API is available for other SetPlayer callers.
Platform event translation, option transitions and the complete simulation/input
owner are still pending; the existing prototype runtime is not the original game.

The original KKMonsterWalk routine and its five authored style definitions are
ported, including foot raycasts, spline/fallback foot arcs, body fitting, gait
transitions, overlapping step timing and deviation state. A 2,560-update native
fixture checks every persistent field and matrix across all orientation/step
flags, original style tables, real/null splines and static/dynamic collision.
NaN cases compare classification. Monster scene registration and complete AI
still need to consume this routine; the prototype game remains separate.

Monster actor initialization/respawn now consumes the original parameter block,
faces the player, resolves collision cells, prepares the hit cell, initializes
walking and applies overrides. A 120-call native fixture checks retained attack
timers/magnet force and stable hit-cell ownership across respawns. The source's
16-word copy into a 14-word parameter block is modeled explicitly: collision
scale x/y overwrite Event.Matrix.i.x/y. This is source fidelity, not verification
of released-binary behavior. Scene registration, shared actor health ownership
and complete AI/tick integration remain pending.

MonsterAI is ported against actual actor, collision and walking state. A native
1,280-update fixture checks spawn gates, visibility/notice transitions, melee and
ranged timers, movement, death/kill switches and painting-event values. Attack
calls are returned as requests; ranged origins capture the pre-movement position
so delayed consumption does not move the muzzle. Player-damage/projectile owner
integration, magnet forces, scene registration and complete tick scheduling
remain pending. This does not make the prototype runtime a faithful game yet.

MonsterMagnetAI now clears all registered forces and processes the first 256
active monsters in original pair order. A 64-phase native fixture (0..512 actors)
checks forces and the next random value, including coincident positions and the
source's unnormalized random-ball fallback. The helper accepts actor references
in registration order and a shared random stream. Scene ownership/registration
and the complete AI/tick owner remain pending.

Monster health now lives in the canonical actor registry instead of a separate
health vector. Persistent IDs survive per-frame registration reordering; removed
IDs are not reused. Projectile damage, splash and restart/continue mutate the
same actor objects, and splash reads their collider positions directly. Frame
setup clears registration while retaining instance-owned actors. Native damage,
splash and checkpoint fixtures still pass; an actual projectile integration test
kills an actor whose persistent ID differs from its registration-list index and
queues the correct explosion. Magnet handling now supports aliased references,
with expanded native pointer-alias/RNG coverage. The monster scene handler and
instance-owned cleanup still need to connect this registry to scene execution.

Monster scene execution (0x11) and overrides (0x15) now use instance-owned actor
IDs, the canonical registry and shared live/death event objects. Living actors
register in source order; in-range actors register collision and leg variables
before scoped execution with an identity parent. Event memory persists across
frames and resets on respawn. A 120-call complete native-handler fixture checks
actor state, dynamic registration, leg variables, event heads and splash effects.
Integration tests cover independent repeated instances, one-shot override
consumption and recursive actor/event cleanup when a parent event expires.
KEvent::Init's lost-chain behavior is distinguished from DeleteChain cleanup.
A collision-world context is required explicitly. Full simulation orchestration,
remaining scene/effect handlers and the original rendered game remain pending.

A simulation owner now assembles the original OnTick order: input and weapon
prefixes, splash/fire, movement/physics/zones/camera/listener, magnet/monster AI,
projectiles, collision endpoints and death continuation. It shares world, actor,
event and random-stream state with scene execution and refreshes placement mouse
counters. Audio/listener and option-update requests retain their original phase
positions for later platform consumption. A whole-function native fixture checks
144 player-only ticks; coupling tests check splash-before-AI and fire-before-move
with same-slice projectile advancement. Mixed populated tick parity, frame-clock/
menu orchestration, platform/audio/render consumption and the original main
runtime still need completion.

SetIf (0x0a), State (0x0b) and Menu (0x0e) scene operators now implement original
switch comparisons, scoped variables, key consumption and menu bounds. LastKey
is canonical gameplay state shared by input and scene execution. A 768-call
native fixture checks child/caller state; scene tests check variable restoration
on errors and key consumption before child menus. The original main loop was
also traced: OnTick runs before the new frame's InitFrame. Frame-clock/root/menu
orchestration and presentation still need to follow that order in the runtime.

Frame initialization now resets the original VM variables/save stack, event
context, camera environments and timing fields, preserving signed slice/jitter
behavior and backwards-time reset. The player main-loop clock ports beat/FPS
calculation and the ten-slice cap. A native fixture checks 768 frame resets and
256 timing updates. Simulation scene preparation installs this state after the
previous-frame tick, then registers document/game events and camera aspect.
Static game events now honor ClearInstanceMem through the same registration
path as document events. Scope-reset and preparation-order tests pass. Root
switching/startup, remaining operators, renderer/audio consumption and replacement
of the prototype main loop remain pending.

## Beta viewport execution progress

The released one-word `0xf0` wrapper now has a distinct Rust execution path:
cache lookup, child execution, target allocation, viewport preparation, a clear
pass, and LastOutput assignment. Tests check full texture extent, the fixed
1024x768 screen letterbox and cache reuse with real output-reference counts.
The reference verifier also runs the actual released PrepareViewport and both
initializers in six bounded cases, in addition to its 24 stubbed wrapper traces.
It confirms clear flags 3 and the wrapper's transparent-black color override.
This supersedes earlier statements that the wrapper itself is unimplemented.
The paint callee's camera/game behavior and complete beta frame equivalence
remain unverified; the main executable still uses the development export.

## Released root initialization audit

Executing the released root (4815) with a postprocessing pool now reaches monster
1183. Preparing its game operators fails with `truncated monster parameters`:
the released class `0x11` packs 26 words (`fffffffffbiiififfbffffffff`), while
`monster_actor::Actor::initialize` consumes the later 29-word development ABI.
This is a concrete remaining blocker for the beta root, before camera or paint
parity can be assessed. The diagnostic used Rust 1.100 release mode and the
independently extracted beta; its harness is retained under `.reference`.
Do not pad these records with guessed defaults: the released monster initializer
and execution handlers need independent mapping before sharing the actor path.

The released monster field-copy prefix now has a separate Rust `InitialCopy`
decoder, checked against 256 executions of the actual x86 instruction slice.
The fixture covers signed zero, subnormal and extreme finite speeds as well as
random records. `KOp::Exec` loads x87 control word 0x143f (24-bit precision,
round toward negative infinity); the speed multiplier follows that rounding,
rather than Rust's default nearest rounding. The decoder preserves the two
three-word vectors and fourteen-word parameter block without assigning guessed
semantics to the vectors. It is a verified prerequisite, not an integrated beta
actor: collision setup, event storage and released behavior still need mapping.

Released monster hit-box local transforms and all eight collision vertices now
have a distinct Rust implementation. A second instruction oracle executes the
actual released InitEuler, vector transforms and cube construction, stopping at
the plane-builder call without replacing any math callees. Its 256 cases compare
all 16 matrix words, three extents and 32 vertex words exactly, including signed
zero and subnormal Euler inputs. The implementation preserves x87's 24-bit
arithmetic with wide exponents, directed rounding, exact cancellation signs and
the observed intermediate f32 stores. This establishes local collision geometry;
released actor ownership, world placement and AI remain separate unfinished work.

Released spawn fields now have a separate `InitialState` decoder. A third oracle
runs the actual spawn prefix through its Event.Init, spline lookup and transform
callees, stopping immediately before FindCell. Across 256 cases it checks the
parent matrix, converted parameter block, both 180-byte event layouts, spline
identity, life, and the exact cell-query arguments. The query uses parent.l with
0.5 added to y; collider type 3 remains on the stack for Collider::Init.
The event translation separately adds the authored
position offset. Token fields represent original identities without storing
native Rust pointers. This remains a construction primitive: FindCell, collider
initialization, dynamic-cell preparation, actor registration and AI still need
their released behavior integrated before the beta root can run.

The released collider uses five particles and ten constraints, rather than the
later development sphere. Its particle/constraint constructor is now ported to
Rust with fixed arrays and index-based links. An oracle executes the complete
released Collider::Init with every original callee, then compares its first
840 bytes against the portable state (relocating identity links at the test
boundary). All 256 cases match, including signed-zero positions, four collider
types and present/absent world cells. The final particle and constraint have
special flags, and the original vector copies and rest lengths are preserved.
The dynamic collision cell, constraint solver, world interaction and actor
integration remain unfinished. The spawn API now correctly calls the trailing
stack word `collider_type`: FindCell's RET 0x10 leaves it for Collider::Init,
so it is not a FindCell mode argument.

The released single-constraint PhysicCons operation is now implemented. It uses
`rest² / (rest² + current²) - 0.5`, preserving the original approximation,
rounding and sequential endpoint writes. Its instruction oracle executes the
actual routine and math callees for 512 cases, including coincident endpoints,
zero rest length, aliased particle links and positions carried across updates.
Both endpoints match bit for bit, including x87's negative indefinite NaN for
0/0; invalid Rust indices fail before modifying state. The source header also
identifies the four particle vectors as Pos, OldCollPos, OldPos and NewPos.
This ports one relaxation operation; scheduling its iterations, particle
integration, world response and the beta actor path remain unfinished.

Released PhysicPart arithmetic and control flow now have a Rust implementation:
position verification/reacquisition, five-particle height leveling, kind-specific
gravity/damping, position-history integration and collision-return adjustments.
A 256-case oracle runs the actual routine and math, controlling only OutsideMask,
FindCell and collision response. It verifies world-call order, exact callback
inputs and every particle state word afterward, including callback mutations.
The final force-add/clear phase is also compared against the actual Physic pass
with particle updates stubbed and no registered constraints/dynamic cells. These
cases include signed-zero and subnormal forces. Bounds/group errors leave the
body untouched and make no world calls. Both operations use fixed storage.
The world-response boundary is deliberately explicit: released collision
resolution, dynamic-cell processing, full tick scheduling and actor integration
remain unfinished; these tests do not establish complete beta physics parity.

Released collision travel normalization and first-plane projection now have
separate Rust primitives. An instruction oracle executes their actual math
slices and compares 256 cases exactly: initial direction/inverse length, impact
point before separation, pushed origin, projected target and next direction.
Cases cover the short-travel cutoff and retained-direction branch. The released
threshold is squared travel 1e-16, and separation is f32 0.0001. The next cast
reuses the initial inverse length rather than normalizing its direction anew.
A short-travel result selects the original driver's fallback; it does not grant
permission to commit the target. Second/third-contact response, world casts,
zone/shot callbacks and driver integration remain unfinished.

Released second-contact math is now ported across all three branches: opposing
planes request a Pos/OldPos reset, aligned planes project again, and the crease
branch constructs the offset intersection line using the original ordered
cross products and determinant arithmetic. The third-cast direction preserves
the released extra inverse-length multiplication. Its 256 instruction-oracle
cases exercise all branches, exact threshold boundaries, and zero remaining
travel; origin, target, direction and reset effects match bit for bit. Shared
plane projection avoids recomputing discarded first-contact direction work.
Final-contact fallback, world casts, callbacks and full driver integration
remain unfinished; this establishes the second-contact arithmetic only.

Released collision finalization now has a Rust implementation for clear,
contact and blocked return bytes (0, 1 and 2). A 256-case tail oracle checks
commit/rollback stores, both modes, kind-zero zone suppression, callback argument
order and callback mutations to particle state and the candidate target. It uses
arbitrary word patterns to verify exact copies, including NaN payloads. Commit
always replaces OldCollPos and Cell; rollback uses the post-callback OldCollPos
without committing the candidate cell. Mode skips the final Pos store on commit
and the Pos/OldPos stores on rollback. The oracle's SkipCell is null: owned-cell
cleanup, cast orchestration, actual zone behavior and driver integration remain
unfinished. This ports the final state transition, not the complete response.

The released three-cast collision response driver now connects normalization,
first/second-contact math, third-cast fallback and finalization. Registry/world
interfaces own skip-cell links, optional shot dispatch, interrupt state, casts
and zones. A complete-routine oracle controls only casts and event/zone callbacks;
all response instructions and math callees execute unchanged. Its 256 cases
compare every particle word, return byte, skip flags, interrupt state and the
full boundary-call trace, including intermediate Pos/OldPos at each cast. They
cover zero through three casts, both modes, interruptions, shot mutations,
opposing resets, absent links/events and callbacks before cleanup. Original
status counts are 64 clear, 64 contact and 128 blocked. Shared first-contact
calculation avoids duplicating the projection in the driver. This supersedes
older driver-pending notes, but actual world cast/zone implementations, registry
integration, dynamic-cell processing and the complete beta game remain pending.

Released hull OutsideMask, IntersectOut and IntersectIn now have separate Rust
plane kernels. Their 512-case instruction oracle runs the actual routines and
math callees, covering empty hulls, radius sign, ties, boundary/parallel rays,
entry thresholds and non-finite inputs. OutsideMask uses z,x,y accumulation and
adds radius; intersection dots use z,y,x. Exit computes `(-1 / dot) * distance`,
while entry divides before negating, preserving directed rounding and stores.
The non-finite cases exposed a shared add bug: opposite infinities were treated
as finite cancellation. It now produces x87's negative indefinite NaN, also
verified by an added actual force-phase case. All beta component checks pass
again. These kernels establish plane queries; additive traversal, subtractive
world resolution and the registry-backed game integration remain unfinished.

Released additive-cell traversal is now implemented with a fixed sixteen-entry
visited array and portable cell indices. Its oracle executes the full released
wrapper and actual hull routines, controlling only terminal CollideRaySub.
Across 128 box-graph cases, final cell, selected contact and exact visited list
match, including the duplicate initial entry, self-links, disconnected cells,
player filtering of OnlyForShot neighbors, cutoff at sixteen entries and ties
between boundary/subtractive hits. The terminal callback runs exactly once;
invalid Rust indices fail before invoking it. False-hit out-buffer contents are
not exposed by the Option contact API. Subtractive/dynamic candidate resolution,
world/registry integration and the complete beta game remain unfinished.

Released CollideRaySub2 now has a Rust single-cell clipping kernel. It preserves
exact mode gating (1, or 9 with monster mode), start-inside interruption, shared
outside-plane rejection, strict candidate priority, the positive 0.0001 margin
check and all-four-word normal negation. Its 512-case complete-routine oracle
uses every original hull/vector callee, comparing accepted/inside outcomes,
interrupt effects and all output bits. Cases include disabled modes, empty
hulls, corner misses, NaNs, boundary zero distance and equal-distance rejection.
The observed split is 452 misses, 40 contacts and 20 inside aborts. World-level
static/dynamic scanning, actor side effects and registry integration remain
unfinished; this kernel does not yet provide complete world collision.

Released static candidate scanning now connects visited additive lists to the
single-cell clip kernel. It preserves list order/duplicates, exact mode gating,
player-versus-other OnlyForShot filtering, entry distance reset and interruption
checks only after an accepted candidate. A 256-case oracle executes the complete
released CollideRaySub with actual clipping/hull callees and zero dynamic cells.
Selected plane/distance, return flag and interrupt state match across null and
player/monster/weapon contexts, reordered lists, misses, inside aborts and prior
interrupt states. The native result split is 65 misses and 191 hits. Invalid list
or cell indices return errors. Dynamic-cell scanning, actor/particle side effects
and integration into the released game runtime remain unfinished.

Released dynamic candidate scanning now follows the static phase through a
borrowed CandidateWorld view. It preserves exact mode/shot-only filtering,
player/monster exclusion for null particle queries, interrupt stopping, collided
kind masks and callback order. Explicit effect boundaries carry particle-hit
and part-box arguments; selector changes feed back into later candidates. The
released part-box handler is a no-op, retained as the default observer boundary.
A 256-case complete CollideRaySub oracle executes actual clipping and Particle::Hit
with null hit-effect slots, observing rather than replacing callees. Output
contacts, flags, masks, selectors and 106 callback records match, including
wrapped kind 35 masks, reflection, static/dynamic competition, absent shot events,
NaN/nonpositive mass and aliased first/primary identities. Native outcomes are
11 misses and 245 hits. Effect allocation/dispatch, general callback mutation,
world/registry ownership and integration into gameplay remain unfinished; this
establishes scan orchestration, not complete beta game behavior.

Released FindCell now has a portable registry-order lookup using the verified
beta OutsideMask kernel. Its 256-case oracle executes the actual FindCell and
OutsideMask routines with zero through six shuffled, overlapping box cells.
The selected identity matches, including boundaries, empty registries and NaN
coordinates. The iterator API preserves caller-owned identities without pointer
storage or allocation. World ownership and runtime integration remain pending.

Released zone-transition masks now have a portable implementation using beta
hull tests. Its 256-case oracle executes the actual zone traversal and hull
callees, comparing all pre-existing enter/leave/collided masks after movement.
Cases cover both cell lists, shared zones, same-cell suppression of the second
list, shot-select suppression, wrapped particle kinds, NaN points and each
inside/outside transition. The implementation retains registry order without
allocating. Zone logic consumption, registry ownership and the full collision
adapter remain unfinished; this establishes the mask-update phase only.

A combined world-cast API now connects additive traversal to CandidateWorld's
static/dynamic scan, preserving current-cell updates, contact selection and the
caller-owned interrupt flag. Candidate-list errors propagate through the terminal
callback. Integration tests exercise portal traversal into a static obstacle and
start-inside abort; an independent verifier runs those same scenarios through
the complete released world cast with no callee stubs and confirms all selected
plane/distance bits, cell identity and interrupt state. Broader dynamic-world
integration, registry adapters, zone logic and the beta game remain unfinished.

Released Particle::Hit's final event-state transition now has a portable helper
for the 180-byte beta layout. Its 256-case oracle runs the actual full hit routine
with null effect slots, comparing all 45 event words across null events,
reflection, monster contact, arbitrary initial state and signed/wrapping times.
Non-reflected or monster hits set Select=1 and Stop's Start=0/End=1; reflection
preserves those fields, while both update ShotTime. Effect allocation/dispatch
must precede this helper when hit-effect slots are present and remains pending.

Released Particle::Hit now constructs effect-event words in fixed Rust storage,
including kind/selection/operator gating, wrapping signed cooldown checks,
monster bypass, source modulation update, position and normal-derived rotation.
A 256-case full-handler oracle replaces only allocation and event-list insertion;
every state/math callee runs original code. It compares all source and emitted
words in 129 emitting cases and 127 skipped cases, including axis/signed-zero
normals, reflection, null events/operators and cooldown equality. The constructor
avoids a heap allocation; the caller still owns event identity and scheduler
insertion. This supersedes the event-construction-pending note, while real slot
lookup, registry ownership and runtime effect insertion remain unfinished.

Dynamic scan effects now have a registry-backed implementation. HitRegistry
owns released shot words, environment/monster operator slots, kind flags and
pending effect events; its DynamicEffects callback invokes the verified hit
constructor and synchronizes selection back into the ongoing scan. Integration
tests cover stopped shots, reflection followed by monster contact, slot choice
and effect ordering. Missing identities, mismatched kinds and stale selections
record a first error without modifying that event; callers must inspect the
error because the existing callback trait has no Result return. The pending
queue still needs connection to the scene scheduler, and the released actor
registry/game loop remain unfinished.

CollisionRuntime now adapts the verified response driver to combined world casts,
HitRegistry effects, zone masks and owned dynamic-cell skip flags. It borrows
geometry and registries rather than cloning them. Integration checks cover clear
movement/history, zone entry, owned-cell exclusion/restoration, wall-contact effect
queueing and stopped-shot zone suppression. Registry-kind and skip-index errors
fail before particle mutation; later geometry/callback errors are reported after
cleanup and may retain prior effects. The adapter does not advance physics or
consume pending effects. Real beta world/actor ownership, scheduler insertion,
dynamic geometry updates and game-loop integration remain unfinished.

CollisionRuntime now implements the released particle world boundary and exposes
a checked single-particle integration method. It uses beta hull containment and
registry-order lookup, then the real response adapter. An independent verifier
runs original collider Init and PhysicPart with actual world/response callees,
no stubs, for the hull-reacquisition/gravity integration case; Rust matches its
exact position-y and history-zero bits. Integration tests also verify registry
errors before body mutation. The caller still selects particle identity for
each step. Constraint/pass scheduling, dynamic geometry updates, effects dispatch
and the complete beta actor/game loop remain unfinished.

A checked single-body pass now schedules all ten constraints, five particle
steps, an explicit dynamic-geometry callback and the final force phase. It
restores the caller's particle identity on success/error and validates registry,
constraint and group links before relaxation. A 32-case oracle runs original
collider Init and complete Physic passes, without callee stubs or registered
dynamic cells. After one through four passes, all five 128-byte particle records
match with relocated links, including randomized forces and exact history bits.
Error tests verify that failed dynamic processing prevents force consumption.
This API is deliberately single-body: multiple bodies require global phase
ordering, and actual dynamic processing must be supplied. The full actor/game
scheduler and dynamic geometry remain unfinished.

The physics pass now preserves global ordering across multiple registered bodies:
all constraint lists, all particle lists, one dynamic-geometry phase, then all
forces. Registrations include each body's particle identities and skip-cell link;
caller context is restored on error as well as success. The single-body entry
point delegates to this shared scheduler. A 24-case no-stub native Physic oracle
registers one through three bodies and executes one through four passes with
random forces; every portable particle field matches after relocating links.
Tests also check all-body prevalidation, phase visibility and failed dynamic
processing without force consumption. The old single-body-only limitation is
superseded. Actual dynamic geometry processing and actor/game-loop registration
remain unfinished, so these checks do not establish full game equivalence.

Released dynamic-cell ApplyMatrix now has a fixed-storage plane update. It
transforms a point on each stored original plane, rotates/normalizes its normal,
recomputes its distance, copies matrix translation into Center, retains the
current matrix and clears Blocked. Its 128-case no-stub instruction oracle runs
all actual math callees for zero through six planes, affine matrices, reflection,
nonuniform scaling and very small/large scales. Every output plane, center and
matrix word matches. More than six original planes is rejected. Dynamic-cell
motion reconciliation against particles, registry geometry ownership and actual
dynamic-phase integration remain unfinished.

Released dynamic motion-delta construction now preserves its scale-aware inverse
algorithm and full four-term matrix multiply. A 128-case oracle executes both
actual routines and their math callees, comparing every delta word for scaled,
reflected, translated and arbitrary finite affine bases. This intentionally
retains the released inverse algorithm rather than substituting a general
inverse with different arithmetic. Particle motion reconciliation, dynamic-phase
integration and the complete beta game remain unfinished.

Moving-cell particle projection now preserves the released full-vector transform,
normalized displacement, hull-entry query and outward separation offset. Its
128-case instruction-slice oracle executes original math/hull callees and checks
proposal bits or the no-motion blocked branch. The oracle stops blocked paths
before the native missing-intersection path could consume unspecified stack
outputs; Rust supplies no proposal for a missing intersection. Callers must still
apply containment/ownership filters, probe collision in mode 1, restore particle
Pos and decide cell acceptance. This is a verified projection prerequisite, not
complete dynamic reconciliation or game-loop integration.

MovingCell now connects pose/delta/projection to particle reconciliation. It
preserves owned-cell and monster/weapon filters, probes in response mode 1,
restores Pos while retaining other probe effects, and commits confirmed geometry
only when every eligible particle clears. A 128-case complete PhysicDCell oracle
uses original math/hull code and controls only the collision probe; all proposals,
particle copies, blocked/mode flags and confirmed/current geometry match. The
verifier uses a fresh emulator to avoid pre-hook cached code from setup. Probe
errors restore Pos and mark the cell blocked. Part-box updates are explicitly
caller-owned and not covered here. Publishing geometry into collision registries,
linking the dynamic phase to scheduled bodies, and game-loop integration remain
unfinished; the missing-intersection path retains the earlier safe limitation.

Released dynamic-frame interpolation now blends the first two basis axes,
reconstructs/normalizes the other axes in the original cross-product order, and
blends translation without clamping the factor. A 128-case actual Fade oracle
checks every output word for endpoint, fractional and extrapolated factors.
MovingCell::reconcile_frame connects this transform to reconciliation; an
integration check verifies accepted confirmed pose and mode updates. Publishing
accepted geometry, part-box processing and wiring the phase into the full game
remain unfinished.

Accepted moving-cell poses now expose borrowed DynamicCell collision views with
registry metadata preserved. Integration tests cast before/after accepted motion
and after blocked reconciliation, proving that scans use accepted planes rather
than an unconfirmed proposal. The view avoids plane allocation/cloning and checks
its plane count. Callers must drop views before mutating moving cells, rebuild
views after each phase and persist updated collided masks in registry metadata.
This provides the geometry-view boundary; the complete owned-world phase driver,
part-box processing and beta actor/game registration remain unfinished.

CollisionRuntime now reconciles a moving cell against registered bodies using
its real mode-1 response adapter. A context-carrying iterator supplies particle
identity and owned-cell exclusion without allocating a flattened particle list.
The original callback-only reconciler delegates to the same implementation.
Integration tests verify a blocked moving hull with actual world collision,
restored temporary positions and restored caller identity/skip context. Existing
native reconciliation comparisons still cover the shared geometry/control path.
Accepted geometry publication remains an explicit caller step; the owned-world
phase driver, part-box updates and complete beta gameplay remain unfinished.

Released collision triangle-plane construction now preserves its degeneracy
threshold and x87 normalization/store order. A 256-case actual MakePlane oracle
compares success and all plane words, including coincident vertices, tiny faces
and random triangles; failed native calls leave output untouched. This returns
the pre-cell positive dot distance, explicitly separate from containment-plane
sign and overlap adjustments. The full released cell constructor and its warped
face/degenerate fallback rules remain to be connected before authored hit-box
geometry can use this path in gameplay.

The complete released vertex-cell constructor now builds fixed-capacity Hull
geometry. A 256-case no-stub Init oracle compares plane count/words, bounds,
center, mode and shot-only flag for boxes, warped faces and degenerate inputs.
It preserves released fallback/split order and 0.005 overlap (doubled for sub
planes), rather than the later development constants. HitBox::collision_hull
connects the verified monster vertices with released mode 1. This supersedes
the full-constructor-pending note; world/actor registration and runtime dynamic
ownership remain unfinished.

Released Actor::spawn now composes InitialState, world-cell lookup, the five-point
body and the monster hit hull in fixed owned storage. A 32-case actual spawn-prefix
oracle executes body/cell construction and PrepareDynamic without stubs, checking
all particle words, local/original hit planes, center, matrices and mode flags.
It covers authored-style and rotated parameters, varied parent translations and
present/absent containing cells. The original hit cell shares body ownership;
PrepareDynamic leaves planes local and marks them unconfirmed until dynamic
acceptance. Missing cells do not invent a life=0 adjustment. Truncated records
fail before world lookup. Scene-instance identity, actor AI/state progression,
world registration and the released game root remain unfinished.

Released actor damage now invokes dormant activation/cell reacquisition before
armor arithmetic, preserves wrapping signed damage behavior and increments the
kill switch as a wrapping byte. A 128-case actual Hit oracle runs activation and
hull lookup without stubs, comparing life, active state, all five body-cell links
and all switches. Cases include dead actors, absent world cells, negative armor/
damage, signed extremes and kill-switch wrap. Invalid positive switch indices
are rejected before mutation. Spawn initializes active=false as in the released
zeroed actor. AI progression, damage dispatch from the scene/world registry and
complete beta root execution remain unfinished.

### Released monster visibility activation

The notice-radius activation gate now matches 144 cases executed from beta
AI addresses 0x81d3d0..0x81d44c. The oracle controls only the visibility result;
activation executes the original world lookup and five particle-cell updates.
Strict comparisons preserve equality and NaN behavior, and deactivation retains
cell links. Damage and visibility share the released activation operation.
This gate is not yet connected to a complete beta AI tick or scene registry.

### Released monster attack cooldown arithmetic

Actor storage now retains the two attack progress values. Active advancement
matches 256 executions of 0x81d458..0x81d4a4, including negative and zero periods,
very small/large periods, and progress below zero and above one. Division and
addition preserve x87 precision and the comparison before the final f32 spill.
Inactive actors retain progress. Attack selection, damage/event emission and
full beta AI/scene integration remain pending.

### Released attack selection

Attack selection and ranged event construction match 384 native executions
of 0x81d4a4..0x81d5dd. Coverage includes equal/unequal cooldowns, NaNs,
the 1.5-unit boundary, missing operators, disabled damage and blocked rays.
The oracle intercepts player damage, visibility, allocation and event insertion,
and executes the original event constructor. Rust returns the resulting effect
for caller dispatch. Full beta AI tick and scene dispatch integration remain pending.

### Released steering scale

The horizontal steering scale matches 256 native executions of
0x81d5dd..0x81d62c, covering both target-distance branches and signed speed
limits. The comparison order and directed subtraction/spill are preserved.
Terrain-height correction and complete beta AI tick integration remain pending.

### Released collider translation

Body translation matches 128 native collider executions at 0x819d55.
Current positions change with directed addition while history, force, weight,
damping, flags and cell links are preserved. Full beta AI movement integration
and terrain-height correction remain pending.

### Released terrain-height correction

Vertical steering correction matches 256 native executions at
0x81d6d4..0x81d70b after the downward cast, including both correction branches.
The gravity/height comparison and directed arithmetic order are preserved.
The complete beta AI tick still needs to supply the actual downward cast result
and dispatch the resulting movement through the scene registry.

### Released attack event modulation

The two attack phase values written to event words 13 and 14 match 128
native executions at 0x81d762..0x81d7b8, including half-cycle boundaries.
This writes persistent event storage. Complete beta AI tick execution and
scene dispatch integration remain pending.

### Released life event modulation

Life ratio and event culling distance 35 match 128 native executions at
0x81d7b8..0x81d7d1. Cases include signed extremes, zero maximum life, infinity
and x87 indefinite NaN. Full beta AI tick and scene integration remain pending.

### Released event position and facing

Event position and rotation words match 128 native executions at
0x81d720..0x81d762. Position follows the main physics particle, and facing
uses the released FPATAN operand order and stored turns conversion constant.
Full beta AI tick and scene integration remain pending.

### Released horizontal player direction

Horizontal direction and distance match 128 native executions at
0x81d36c..0x81d39c, including distances below the normalization threshold.
The vector points from player to monster, with the released short-vector
fallback. Full beta AI tick and scene integration remain pending.

### Released steering displacement

Displacement construction matches 256 native executions at
0x81d5dd..0x81d644. The scale is applied before adding and subtracting the
main particle position with intermediate f32 spills. Full beta AI tick
and scene integration remain pending.

### Composed released actor tick

The beta actor tick now restores its event matrix, calculates player direction,
updates visibility and activation, advances and selects attacks, applies steering
and terrain correction, translates its body and publishes event pose/modulation.
World lookup, visibility and ground queries are supplied through an explicit
world interface. Integration checks cover activation/query order, movement,
persistent event updates and dead-actor early return. All nine focused native
fixture test targets pass. Whole-tick native comparison, concrete collision-world
adapter and scene registration/dispatch remain pending.

### Whole released actor tick comparison

Thirty-two complete native actor ticks at 0x81d33e now match Rust particle
positions/history, cell links, all 45 event words, cooldowns and activation.
The oracle executes native spawn, world lookup, AI and translation, intercepting
visibility and ground casts. Cases include blocked visibility and absent cells.
These first ticks do not reach attack emission. Multi-tick attack comparison,
concrete collision-world adapter and scene registration remain pending.

### Consecutive released actor ticks

Thirty-two cases of one to sixteen consecutive native ticks match particle
positions/history, cell links, all event words, cooldowns and activation.
Attack damage is disabled in these runs. Concrete collision-world integration,
attack dispatch over longer runs and scene registration remain pending.

### Whole-tick fixture coverage correction

The first whole-tick fixtures coupled clear visibility to missing world cells,
so no actors activated. Visibility now varies independently from cell presence.
Both fixtures were regenerated and the Rust comparisons pass with eight active
cases each, covering actual movement and cooldown advancement. This supersedes
the earlier implication that the first fixture versions exercised active ticks.

### Repeated released melee ticks

Thirty-two cases of up to sixteen consecutive native ticks now include melee
attacks. Actor state and accumulated damage match, with active cases producing
10 to 70 damage. Player damage is intercepted at the native service boundary.
Ranged multi-tick comparison and concrete scene/world integration remain pending.

### Repeated released ranged ticks

Thirty-two cases of up to sixteen consecutive native ticks match actor state
and every emitted ranged event. The fixture contains 64 events. Allocation and
event insertion are intercepted, with native event construction preserved.
Concrete collision-world adapter and scene registration/dispatch remain pending.

### AI collision-world adapter

AI world queries now use the shared additive/subtractive/dynamic collision
registry. Visibility starts from the player cell, while downward casts use the
actor particle cell and temporarily exclude its owned hit cell. Runtime errors
are retained in the collision adapter. Hull integration checks cover clear and
blocked visibility, floor distance and owned-cell mode restoration. Actor scene
registration, attack dispatch and complete runtime integration remain pending.

### Released attack operator identity

The initializer field at actor+0x1e0 is now named attack_operator. The released
call at 0x81d863 reads link operator zero through 0x80e11f, and ranged event
creation copies that operator to the new event. The earlier spline_token name
was misleading. Initialization and repeated ranged-event comparisons pass.

### AI attack dispatch through collision runtime

CollisionRuntime::tick_actor now runs AI against its collision registry and
applies melee hits through player Inventory::hit, returning the hurt sound.
Ranged events enter the shared pending-event queue. Integration checks cover
actual floor queries, player health changes, hurt sound and ranged queue output.
Scene actor registration and pending-event scheduler integration remain pending.

### Released event word 44 semantics

The final event word is CullDist, consistent with the 180-byte released layout.
The AI writes 0x23 (35) there, while Select remains at word 12. Comments and
verification naming now identify culling distance correctly. Stored bits are
unchanged. This distinction matters when bridging events to the scene scheduler.

### Released event conversion for scene scheduling

Event::from_released_words maps the 180-byte event payload into scene event
storage after pointer tokens have been resolved to portable identities. Native
actor-event fixtures verify pose, scale modulation, matrix and culling fields
through scheduler insertion and variable access. Token ownership, pending-queue
publication and beta actor scene registration remain pending.

### Released pending-event publication

The scheduler can resolve and publish a released-event batch in queue order.
Resolution failure preserves the queued payloads and scheduler, allowing retry.
Successful publication clears the queue. Checks cover resolution failure, retry,
identity mapping, ordering and repeated publication without duplicates. Concrete
scene-owned token maps and beta actor registration remain pending.

### AI event pipeline integration

The collision-runtime integration now follows a ranged attack through real hull
queries, pending-event storage, token resolution and scene scheduler insertion.
Operator/actor identities and emitted rotation/translation survive publication.
The released initializer reads its attack operator from link zero (0x80e11f),
while its living scene executes input zero (0x80e106). Concrete scene-owned
registration and token maps remain pending.

### Ordered multi-actor AI dispatch

CollisionRuntime ticks actors in registration order with each owned dynamic cell
selected for ground exclusion. Owned-cell bounds are checked before any actor
advances, and the caller's skip-cell context is restored on return. Integration
checks cover combined melee damage, returned sounds, context restoration and
invalid registrations without partial advancement. Scene-owned registry and
frame-loop integration remain pending.

### Physics over actor-owned body storage

The global body phase accepts mutable body containers, including released actors,
so actor bodies remain in their persistent storage across constraint, particle,
dynamic and force phases. Existing body callers retain their interface behavior.
Native single/multi-body checks and an actor-storage integration check pass.
Scene registry and frame-loop integration remain pending.

### Dynamic reconciliation over actor storage

Moving-cell reconciliation now accepts the same body containers as the global
physics phase. The actor-storage integration runs reconciliation inside the
dynamic callback, while existing native reconciliation checks continue to pass.
This preserves persistent actor storage through the physics and dynamic phases.
Scene registration and complete frame-loop integration remain pending.

### Full regression run after actor-storage integration

At commit 75aec4b, all test targets compiled and the complete cargo test run
finished successfully with 426 passed, zero failed and 29 ignored across 200
result targets. This verifies the current test coverage. Full released-game
playthrough, scene registration and runtime parity remain outstanding.

### Owned released static collision registry

StaticWorld owns released hulls reconstructed from source vertices and builds
connectivity from their released bounds. Additive neighbors use additive-registry
indices, while subtractive and zone lists retain source-cell indices. Borrowed
views feed the existing cast runtime. An interleaved-cell integration check covers
index mapping, neighbor lists and a real subtractive cast. Scene-world extraction
and frame-loop ownership remain pending.

### Released static world from scene documents

StaticWorld::from_document builds scene geometry through the existing world
builder, then reconstructs released hulls and connectivity from source vertices.
It requires the beta document layout. The independently extracted beta level at
root 4357 built successfully and passed registry bounds checks in 26.25 seconds.
Scene actor registration and the complete beta frame loop remain pending.

### Persistent released actor registry

Released actors now have stable nonzero event tokens, shared persistent handles
and an ordered per-frame registration list. Failed spawn does not consume an
identity. Removal clears registrations and identities are never reused. Registry
checks cover frame resets, duplicate registration order, retained handles and
removed identities. Scene instance ownership and frame-loop wiring remain pending.

### Ordered access to registered actor storage

Registry::visit_active releases each actor borrow before advancing, preserving
repeated registration order over shared persistent storage. Borrow conflicts
return an error. Checks verify ordered mutations, repeated identities, conflict
reporting and subsequent access after the borrow is released. Runtime phase and
scene-instance integration remain pending.

### Physics phases over registered shared actors

CollisionRuntime::step_registered runs global constraints, particles, one dynamic
callback and forces through persistent registry handles. Repeated registrations
remain ordered without simultaneously borrowing the same actor. Both body paths
share registration validation and restore caller particle/skip-cell context.
Integration and native body-pass checks pass. Scene registration and full frame
wiring remain pending.

### AI over registered shared actors

CollisionRuntime::tick_registered validates registrations, ticks shared actors in
registration order and dispatches their attacks through inventory/pending events.
The caller's skip-cell context is restored. Integration checks advance the same
registry actors through physics and AI with real hull queries. Scene instance
registration and the complete game frame loop remain pending.

### Released monster scene execution

Beta operator 0x11 now creates a 26-word released actor in persistent instance
storage, reads its attack operator from link zero, registers living actors and
executes living input zero with the converted event. Replay preserves actor
identity and life. Scene instance cleanup removes actors from the corresponding
ABI registry, and frame resets clear both registration lists. Existing monster
scene tests and the new beta replay check pass. Spawn cell lookup currently
maps the scene world's additive identities. Released dynamic-view registration,
physics/AI frame scheduling and feedback of scene-modified matrices remain pending.

### Released scene matrix handoff

Living monster execution captures the actor matrix into its moving collision
cell before executing the living input. The resulting scene event matrix is
copied back into the released payload for the following AI tick. The beta replay
check verifies the collision matrix handoff, and all monster-scene tests pass.
Dynamic-view registration and complete frame scheduling remain pending.

### Full released scene-root execution

The independently extracted beta root 4815 now executes successfully through
scene generation and postprocessing setup and registers released actors. The
check builds the level world at root 4357 and runs the full root on a 32 MiB
test thread stack. The default test-thread stack overflowed during traversal.
The reproducible check passed in 11.48 seconds. GPU parity, runtime physics/AI
frame wiring and complete game playthrough remain pending.

### Inactive released actors excluded from physics

The native scene handler registers living monsters for AI but adds colliders
only for active monsters. Registered physics now skips inactive actors during
validation, constraints, particle updates and force application. An integration
check confirms an inactive actor with no body registration retains its entire
body and pending force. Active physics checks and Clippy pass.

### Scene-owned released collider roster

Scene execution now records released collider tokens separately from living AI
registrations. Only active actors enter this ordered collider list. Frame start
and failed execution clear it alongside other dynamic registrations. Scene tests
cover inactive spawn, active replay and subsequent deactivation while retaining
the living AI registration. Runtime consumption of this roster remains pending.

### Released collider frame capture

ActorFrame captures scene collider geometry separately from mutable actor
storage, assigns stable particle identities with a caller-selected base, and
builds body-to-owned-cell mappings. Borrowed collision views remain usable while
actors mutate. Checks cover reordered actors, inactive body mappings, invalid
tokens and identity overflow. Runtime frame ownership and dynamic reconciliation
publication remain pending.

### Dynamic probes against registered actor bodies

CollisionRuntime::reconcile_registered borrows active physical actor bodies for
the existing moving-cell probe phase and releases them afterward. Physical
identities are unique in the probe set, while AI/physics registration traversal
retains its order. The registered physics integration invokes reconciliation
inside the dynamic callback and passes. Complete frame ownership and accepted
geometry publication back to scene actors remain pending.

### Accepted actor geometry writeback

ActorFrame retains collided masks across rebuilt views and can publish accepted
moving-cell geometry back to persistent actors. Publication resolves and borrows
all target identities before changing any geometry. Checks cover mask-size
validation, retained masks and geometry writeback preserving actor life. Full
frame-stage composition and game-loop integration remain pending.

### Captured collider roster through physics phases

Physics can now consume an explicit scene-captured collider token list. All
constraint, particle and force phases use that same list even if activation
changes later. The convenience registered-body path captures active tokens once
and delegates. Checks verify a selected body advances despite later inactive
state and a newly activated body outside the roster remains untouched. Full
frame-stage composition remains pending.

### Released frame ordering evidence

The beta main update calls global physics at 0x81e508 (target 0x81c728), then
weapon/player work at 0x81e522, then loops registered monster AI at 0x81e571
(target 0x81d33e). Frame composition must therefore retain the scene-captured
collider roster through physics and run monster AI afterward.

### Sequential accepted-geometry reconciliation

ActorFrame can reconcile captured cells in order, rebuilding borrowed collision
views after each accepted update. Collision masks, zone interactions and runtime
errors pass through the probe runtimes, and accepted geometry is published to
persistent actors. Integration checks verify acceptance and persistent writeback.
The complete physics/AI frame wrapper and game-loop invocation remain pending.

### Owned actor-frame stage composition

ActorSimulation owns the released static world, hit registry and zone masks.
Its step captures scene colliders, runs global physics and sequential dynamic
reconciliation, then ticks monster AI against accepted geometry. The integration
check covers inactive activation, collider acceptance on the following frame,
melee health/sound output and dead-actor behavior. Native actor tick comparisons
and collision runtime tests pass. Player/projectile physics, game-loop invocation,
scene token resolution and full playthrough remain pending.

### Scene-owned released actor frame stepping

Execution::step_released_actors consumes its persistent registry and captured
collider list, applies the actor frame simulation to game inventory, resolves
encoded document operator/spline and actor tokens, and publishes emitted events
into the scheduler. A scene-level check covers ranged publication and subsequent
collider acceptance. Beta test executors are rebuilt after document edits so
cached parameters reflect the intended fixture. Main application invocation and
player/projectile integration remain pending.

### Extracted beta scene with repeated actor-frame simulation

The full beta root now runs four actor-frame steps and scene replays using the
authored player start, gravity, damping and stair-height settings. The run
exercises active physics (two colliders observed) and verifies finite particle
state. Its reproducible ignored test passes with the larger traversal stack.
This is a short stationary integration run. Player/projectile stepping, main
application invocation, visual parity and full playthrough remain pending.

### Separate released physics and AI stages

ActorSimulation exposes physics and think stages with accepted ActorFrame
geometry passed between them. The combined step delegates to these stages. This
allows the main loop to preserve intervening zone/player work. Integration checks
verify physics does not activate or attack, followed by AI activation/damage.
Main-loop invocation and player/projectile parity remain pending.

### Main simulation owns beta actor world

Execution retains its document layout, and the main Simulation constructs a
released actor world for beta documents. Root replacement rebuilds that world
and clears its pending actor events. Checks cover layout selection and scene
replacement, alongside existing simulation and monster-scene tests. Main tick
stage invocation and player/projectile parity remain pending.

### Released actor stages invoked by main game tick

The main simulation now invokes released actor physics before zone/player
postprocessing and monster AI afterward. Authored gravity/damping/height settings
and current player/cell state feed the stages. Emitted events use the shared
scene publication path, and hurt sounds enter tick audio output. A beta main-tick
check verifies activation and ranged publication. Existing simulation, scene and
actor-frame tests pass. Player/projectile ABI parity, shared zone semantics and
full released-game playthrough remain pending.

### Released actor zone masks reach shared zone actions

The main tick now transfers released actor enter/leave/collided masks into the
source-indexed world entries before zone logic runs. The transfer preserves
player masks, consumes actor masks once, and rejects mismatched world sizes
before changing either side. A zone integration check covers an actor-entry
switch action, repeat consumption, player-mask preservation and failed transfer.
Full beta zone-logic parity and complete released-game playthrough remain
unproven; this connects the released collision masks to the existing zone logic.

### Released zone-action rules verified against machine code

A Unicorn oracle runs the released executable's zone loop at 0x81cea2 through
0x81d1e0, capturing 1,664 zone phases across action codes, switch conditions,
interaction masks, inventory values and respawn timers. Beta documents now use
the released timer behavior (no event-instance reset on expiry), select collected
weapons only when their previous amount is zero, and apply the released x87
round-down arithmetic to speed-up actions. The development rules remain covered
by their existing C++ oracle. The released function's subsequent player/dynamic
weapon-hit dispatch is outside this fixture and still needs integration.

### Released dynamic weapon-mask damage dispatch

Actor frames now consume weapon collision bits 8..15 in ascending weapon order
and invoke the released actor Hit implementation before AI in the main tick and
the scene-owned actor step. Repeated references to a shared actor combine their
masks and apply each weapon once. Activation uses the released additive hulls;
lethal damage updates the authored kill switch. Checks cover armor, repeated
references, mask consumption, lethal hits and retry after a conflicting actor
borrow. This follows the dynamic dispatch loop at 0x81d218..0x81d29e; released
player/projectile body registration and player weapon-hit dispatch remain
incomplete, so ordinary released projectile impacts are not yet end-to-end.

### Released player damage checked directly

A native oracle now executes Player::Hit at 0x819cf4 for 1,984 cases: the full
12-by-12-by-12 grid of signed edge values for life, armor and damage, plus 256
seeded random cases. The existing shared Inventory::hit matches resulting life,
unchanged armor, hurt-sound gating, slot and volume. The oracle also captures
the released audio call's zero sample offset and 44,100 Hz playback rate. Only
the audio output call is intercepted. This verifies the health primitive; it
does not establish released player-body or projectile registration parity.

### Released player placement preserves null-cell positions

Tracing released SetPlayer at 0x81cb71..0x81cc06 shows FindCell feeding the
five-particle constructor directly, without the later source's first-cell
fallback. Beta simulation creation and subsequent checkpoint/restart placement
now preserve the requested position when no released additive hull contains it.
Containment uses released hull planes, returning source-world cell indices to
the existing motion layer. Development placement retains its fallback. A check
covers outside/inside placement, stable hit-cell identity and preserved movement
timers. Released five-particle player stepping and hitbox registration remain
pending; the current motion integration still uses the development collider.

### Released pre-physics player input translation ported

The released instructions at 0x81e399..0x81e4a1 now have a Rust counterpart in
beta_player::Control::prepare. The function translates all five current particle
positions using the authored side/forward axes, consumes speed-up, applies the
released ground-height correction, updates air time and emits landing audio.
A native oracle executes the original vector operations and body translation;
128 captures cover different axes, gains, gravity, slice counts, ground/air
conditions and unordered comparisons. Particle positions and control fields
match exactly; histories, forces and constraints remain unchanged. This input
stage is not yet wired into live Motion: player body persistence, shared physics
ordering, ground queries and released hitbox registration remain necessary.

### Player bodies can share registered actor physics phases

CollisionRuntime::step_body_roster accepts owned prefix bodies and their stable
particle registrations before the scene-captured actor roster. Both groups now
complete constraints before either group advances particles; dynamic callbacks
then see both updated groups, and forces apply afterward. Existing actor-only
calls delegate to this path with an empty prefix. A synthetic collision-world check
covers player/actor particle movement, phase-visible state, force application,
context restoration and validation before mutation. Live player ownership,
dynamic-cell reconciliation against the player and Motion integration remain
pending; this supplies the common phase runner without changing live movement.

### Owned actor frames accept player bodies through reconciliation

ActorSimulation::physics_with_bodies now registers prefix bodies (including
player kinds 1/2) alongside captured actor particles and carries them through
the common phase runner and every dynamic reconciliation probe. Particle-ID
collisions fail before shot registration or body mutation. Reconciliation uses
borrowed body references and preserves native owned-cell exclusions and the
monster-hull rule that skips non-weapon particles. Checks cover a player and
actor advancing together, ordinary moving-hull rejection against player
particles, monster/owned-cell exclusions and context restoration. Live Motion
still needs persistent released body ownership, ground queries and hitbox
registration before this path can replace the development player collider.

### Released player body ownership and input-state bridge

Beta Motion now owns a persistent five-particle player body with IDs 0..4.
Placement reconstructs it using released containment and additive-local cell
indices, while the public motion collider retains source-world cell indices.
Checkpoint placement resets particle history/forces without resetting movement
timers or replacing the shared hit-cell identity. Motion::prepare_released maps
input axes and control fields into the native-verified pre-physics translation,
then copies landing/air/jump state back. Checks cover interleaved world indices,
null cells, persistent force state, placement resets and pre-physics publication
ordering. The main tick still invokes development player movement; released
ground queries, post-physics publication and hitbox integration remain pending.

### Main beta tick advances the released player body

Beta ticks now use a released downward ground query, prepare player input,
advance persistent player particles alongside scene actors, and publish the main
particle position/cell before zone actions and AI. Publication validates local
cell IDs before changing motion and converts them back to source-world indices.
Player hit-cell matrices are prepared before physics using the released 0.85
offset. Checks cover ground clearance including native hull overlap, null/error
cells, publication validation and consumption of player forces by the main tick.
The player ground query and released frame still lack ordinary scene dynamic
hulls (doors/platforms), and full released player-hitbox/projectile registration,
footstep/camera rounding and complete playthrough verification remain pending.
An opt-in extracted-data check also ran full root 4815 and four main gameplay
ticks at the authored start position, verifying finite player particles,
published player state, registered player shots and active actor colliders.
It forces the options switch off and does not establish a complete playthrough.

### Scene-owned hull bridge feeds released player ground rays

SceneCell captures a stable scene dynamic-cell handle, rebuilds local planes
with released hull arithmetic, and exposes tagged collision views. It can
publish accepted collision planes/transforms, logic masks and blocked state
back to the same scene instance. Player mode conversion and monster ownership
are explicit; borrow conflicts fail without replacing the instance. Main beta
player ground rays now include these scene-owned hull views, while preserving
the native null-particle exclusion for player and monster hulls. Checks cover
ordinary hull clearance, tagged exclusions, native plane capture and accepted/
blocked publication. Scene hulls still need to join particle casts and native
dynamic reconciliation; their interpolation remains on the development path.

### Scene hulls participate in released particle physics

Actor frames now retain scene-owned hull snapshots alongside actor geometry.
Particle casts and dynamic reconciliation include both groups, write accepted
scene planes/blocked state back to the live instances, and preserve their
collision masks. Beta main ticks propose scene transforms with the released
interpolator and select the player's owned scene cell; development interpolation
is bypassed on that path. AI queries use the same accepted scene hulls. A wall
check verifies that scene geometry stops released player particles, and the
extracted root/main-tick check still passes four ticks. Exact interleaving of
actor and scene registry entries, duplicate scene identities, part-box effects
and player/scene-owner weapon damage dispatch still need parity work.

### Mixed dynamic registration order retained by main beta physics

Scene execution now records player/scene and active actor hull entries in their
encounter order, including actor live-input geometry, and resets the roster on
frame replay and error cleanup. Actor frames validate the mixed order, remap
owned-cell indices and collision masks, and use it for casts and reconciliation.
The main player selects its owned cell from that roster. Checks cover player →
actor → live geometry, inactive/dead actor transitions, replay cleanup, ordered
mask mapping and invalid order rejection. Duplicate pointer identities, complete
native mixed-registry traces, part-box effects and weapon damage dispatch to
player/scene-owned actor hulls remain unverified or incomplete.

### Captured weapon masks dispatch to player and scene-owned actors

The zone-tail dispatcher now applies player enter/collided weapon bits first,
then visits actor and scene-owned actor hitboxes in mixed roster order. Repeated
references to one physical cell combine masks and hit once; distinct hitboxes
owned by the same actor remain separate hits. Actor and scene-instance borrows
are validated before health changes, allowing retry without duplicate damage.
Consumed masks clear both frame metadata and live scene instances. Main tick
hurt sounds enter the audio result. Checks cover player damage/audio, alias
suppression, distinct hitboxes, kill-before-heal ordering and borrow retries.
Native player/actor damage primitives were independently verified earlier;
released projectile registration and end-to-end impact parity remain pending.

### Standalone released projectile particle primitives

Particle::new now matches the released initializer at 0x81a939, including copied
positions, velocity-derived history and reset flags/mass. A no-stub native
fixture covers 256 inputs, signed zeros and non-finite values. Particle::step
and apply_force share the existing body arithmetic rather than duplicating it;
a projectile variant of the native PhysicPart/force oracle covers 256 cases
across kinds 8..15. The existing particle fixture regenerates with its original
hash. CollisionRuntime::step_single validates registration before mutation and
uses real world/zone/hit callbacks. Focused primitive and runtime tests pass.
Scene Physic mode-0 particle ownership, registration order and projectile event
lifecycle are not yet connected; eight-particle projectile boxes also remain.

### Scene Physic mode-0 point state and ownership

A native oracle now runs released Physic mode-0 initialization, intercepting
only instance allocation, for 128 authored-parameter/matrix combinations. The
Rust Point state matches particle mass/kind, rotated velocity history, event
references and player-skip flags. Beta scene instances retain that point across
replay, register it in encounter order alongside actor bodies, and execute child
geometry at its current particle position. Checks cover persistent state, event
ownership, mixed actor/point registration and error cleanup. The current released player
cell is copied through scene context without a new containment search. These
points are not yet advanced by the main shared physics pass; event-state sync,
projectile effects/lifetime integration and eight-particle boxes remain pending.

### Mixed actor/point physics phase runner

CollisionRuntime::step_mixed_roster now accepts registered standalone points and
the scene-captured actor/point order, after player prefix bodies. It validates
all identities, owned-cell indices, point borrows and roster completeness before
mutation; constraints precede all particle movement, dynamic callbacks precede
all force additions, and caller context is restored. Actor-only entry points
now delegate to this runner. Checks cover point-before-actor ordering, delayed
force application, missing identities, duplicate slots and conflicting point
borrows, alongside existing native particle fixtures and actor frame tests.
The main frame still needs point-event registration/synchronization and point
participation in dynamic reconciliation before selecting this mixed path.

### Point projectiles participate in moving-hull reconciliation

MovingCell now supports sequential borrowed-particle visits, allowing repeated
registrations of one particle without simultaneous mutable borrows. Existing
iterator entry points use that same implementation and still match the native
reconciliation fixtures. The runtime and actor frame can reconcile against a
mixed actor/point roster, retaining probe order, owned-cell exclusions, position
restoration and accepted/blocked publication. Checks include a point rejecting a
moving hull, live scene blocked-state publication, repeated visits and shared/
exclusive borrow rejection before mutation. Native registration append routines
were inspected to confirm repeated pointers are retained. Main-tick point event
registration/synchronization and end-to-end projectile impacts remain pending.

### Projectile hit state synchronizes with live scene events

Scene events can snapshot/apply the five fields used by released Particle::Hit
without replacing matrices, references, animation intervals or other event data.
The bridge matches all existing native hit-effect captures. HitRegistry binds
multiple particles to one scene event and immediately mirrors stop/reflection
state among those aliases, preventing duplicate effects and honoring shared
shot timestamps. Point registration captures its owning event, and batch commit
validates every live event borrow before applying any changes. Checks cover
shared stop/reflection gates, unrelated field preservation and borrow retries.
Main-frame point preparation, effect-table/time setup and mixed-step invocation
are still required before live projectile impacts use this bridge.

### Owned frame accepts point projectile physics

ActorSimulation accepts explicit point registrations and mixed physics order,
steps projectiles against scene walls and reconciles them with moving hulls.
Integration checks cover stop/reflection event updates, impact effects, identity
overlap rejection and retirement of stale shot/event bindings while retaining
queued effects. The released general-particle gravity constant is restored from
the native initializer. Main-tick point preparation, effect tables, time setup
and event synchronization remain pending; these checks do not establish full
projectile gameplay parity.

### Point roster preparation validates before binding

A frame-preparation helper assigns compact point identities after the caller's
reserved body range, preserving repeated Rc registrations as one particle. It
captures owning event state and player-cell exclusions in a staged registry,
then replaces bindings only after all point/event borrows and identity arithmetic
succeed. Tests cover aliasing, stale-event retirement, missing-event retries,
exclusive-access conflicts and overflow without losing prior bindings or queued
impact effects. Main-loop invocation and native hit-table/time setup remain pending.

### Main tick advances released point projectiles

The beta main loop now prepares the scene's point roster after reserving actor
particle identities, uses the recorded actor/point order for mixed physics,
applies the player's dynamic-cell exclusion and commits shared hit fields back
to owning scene events before zone processing. Impact operators use the existing
one-based released-reference encoding; weapon reflection flags and the game tick
counter feed the native hit logic. Native OnTick at 0x81dd74 increments the same
0x15540 field read by Particle::Hit at 0x81a9ef. Queued impacts are published through
the released-event scheduler alongside actor effects. A main-tick integration
check covers wall collision, stop/reflection lifetime, hit timestamp and actual
impact-event publication. Projectile box modes, complete weapon playthroughs and
full mixed native frame-trace parity remain unverified or unimplemented.

### Released Physic hulls preserve native monster ownership

A new native capture executes complete Physic mode-2/mode-3 handlers, replacing
only instance allocation and child execution. Across 32 inputs with varied
transforms/scales and flags 0x02/0x12/0x22/0x32 and 0x03/0x13/0x23/0x33, mode 2
registers one dynamic hull with a null monster owner; mode 3 registers none.
The Rust scene handler incorrectly inherited later-source flag-0x10 monster
assignment. That assignment now applies only to the development layout. The
scene regression checks native ownership and counts during live actor events
and repeated frame replay. This does not implement the missing mode-1 box body.

### Released eight-particle box initialization

BoxBody now constructs the released PartBox's eight corner particles and sixteen
edge/diagonal constraints using the native corner/link order and floating-point
arithmetic. A new oracle executes PartBox::Init at 0x81aaa4 without replacing
callees; 128 captures verify every particle word and normalized constraint word
across varied transforms, velocity, nullable cell references, negative scales
and degenerate zero axes. The Physic handler's mass/kind/event assignment, box
registration, constraints/physics participation and child-pose reconstruction
remain required before mode-1 box projectiles are functional in gameplay.

### Box child transform reconstruction

BoxBody reconstructs the released child matrix from the eight current particle
positions: it accumulates four directed edges per basis axis, normalizes each
axis independently and averages all corners for translation. A native oracle
executes PartBox::MakeMatrix at 0x81abbd without replaced callees; its 256 cases
include arbitrary deformed positions and collapsed boxes. The implementation
preserves the released negative-indefinite NaN bits on collapsed axes. Box
scene ownership, registration and physics integration remain outstanding.

### Persistent scene box projectile ownership

The beta Physic mode-1 handler now initializes and retains BoxProjectile state,
replays the same Rc particle storage on subsequent scene frames and reconstructs
the child transform from that storage. Per-frame box registrations clear on
normal frame reset and execution errors. A new native oracle follows the actual
mode-1 handler initialization, stubbing only instance allocation; all particle
and constraint words match 128 captures, including rotated speed, mass, kind,
nullable event ownership and player exclusion. Scene tests cover persistence
and error cleanup. Box registrations do not yet join the mixed physics roster:
constraints, particle stepping, force application, reconciliation and shared
event binding for all eight particles are still required for box gameplay.

### Box constraint solving shares released arithmetic

The released PhysicCons implementation now operates on a particle slice and is
shared by five-particle actor/player bodies and eight-particle BoxBody storage.
Box constraint lookup validates both slot and particle indices before mutation.
The box checks reuse all 512 native PhysicCons captures across all sixteen
constraint slots and all eight particle indices, including aliased links and
degenerate arithmetic. Existing collider and full body-pass fixtures remain
passing. Mixed-roster box constraint scheduling and particle/event integration
are still pending.

### Complete released box physics pass

CollisionRuntime can execute an eight-particle box pass with upfront identity,
kind, skip-cell and constraint-index validation, followed by all constraints,
particle/world steps, a dynamic callback and deferred force additions. Caller
particle/skip context is restored on success and failure. A new oracle runs the
complete native Physic routine for one box and a real additive hull without
replaced callees; 32 captures with one through four passes match every particle
word. Checks also cover rejected identity/constraint registrations before
mutation and skipped force addition after a dynamic-phase error. Integrating
boxes with the interleaved scene roster and shared event aliases remains pending.

### Shared point and box event preparation

Projectile preparation can stage point and box registrations together, assigning
eight consecutive identities per distinct box while retaining repeated box
references as aliases. Every corner binds to the owning scene event through the
same hit-state bridge as point projectiles, so an impact on any corner updates
all aliases immediately. Staging commits only after borrow, event and identity
range validation. Checks cover a point plus eight box corners sharing one event,
a single queued effect, repeated box references, missing events, borrow failures,
overflow and the last valid eight-ID range. Point-only callers delegate to this
preparation path. The main loop still needs box roster scheduling and dynamic
reconciliation before it can select combined point/box preparation.

### Boxes join the mixed runtime roster

PhysicsRoster accepts box registrations and PhysicsSlot::Box entries. Upfront
validation covers roster completeness, box borrows, particle identities/kinds,
cell exclusions and constraint indices. Constraint solving, particle stepping
and deferred force addition follow the interleaved actor/point/box order within
each phase. Moving-hull reconciliation visits all eight corners, retains repeated
box registrations without overlapping RefMut borrows and honors owned-cell
exclusions. The mixed runner matches all 32 complete native box-pass captures;
checks also cover hull rejection, position restoration, repeated box references
and borrow conflicts. ActorSimulation and the main scene tick still pass an
empty box roster until their frame identity/order wiring is completed.

### Main tick includes scene box projectiles

Physic mode-1 scene execution records box slots in encounter order. The main
loop prepares point and box bindings together, reserves their identities after
actor bodies and passes both rosters to ActorSimulation. Owned-frame identity
validation distinguishes every box corner while allowing repeated registrations
of the same physical corner; metadata retirement retains live box event aliases.
Boxes now participate in all mixed physics and moving-hull phases. Main-tick
checks cover point/box stop and reflection, shared timestamps, a single impact
effect at the wall and rejected cross-projectile/corner identity overlaps before
mutation. Scene replay checks verify box-order registration. Full weapon
playthroughs, actor-live event synchronization and complete native mixed frame
trace parity remain unproven.

### Actor scene replay retains live hit fields

Released actor scene replay previously replaced its live event from stale actor
words, discarding projectile stop/reflection state. It now imports the five live
hit fields before republishing actor pose/animation data and captures those
fields again after executing the live input. A regression reproduces the stale
reset and checks retained timing, selection and modulation alongside updated
actor translation and culling distance over repeated frames. This fixes replay
persistence; the actor's raw event words still do not share the scene allocation,
and immediate raw-word synchronization after main-tick projectile hits remains
to be audited against complete native frame traces.

### Main-tick hit commit updates owning actor records

Released scene actor creation records the identity of its live event allocation.
The main-tick hit commit resolves that exact ownership before copying the five
hit fields into actor event words; a separate attack event referencing the same
monster does not qualify. Scene and actor mutable borrows are validated for the
entire batch before any commit. Tests cover immediate raw-word updates, an actor
borrow preventing both scene and actor mutation, retry after release and foreign
attack-event isolation. The general scene-only synchronization entry point
continues to support standalone registries. Full shared-allocation/event-VM
parity beyond the hit fields and complete native mixed frame traces remain
unverified.

### Refreshed whole-project verification

The full test build found an old game-event fixture constructing zone runtime
state directly after its layout discriminator became private. The fixture now
uses Runtime::new and supplies the captured respawn timers through the public
field; its native event expectations remain unchanged. The complete non-ignored
suite passes 479 tests with no failures and 32 ignored checks across 210 result
blocks. Clippy with warnings denied passes across all targets. These checks
do not verify ignored GPU/native/long-running gates or original-game completion.

### Current release measurement after projectile integration

The Rust 1.100.0 release build succeeds and its `--bench` path completes. The
macOS ARM64 executable is 1,300,656 bytes (SHA-256
`d24ea889e903d90ff57aabf028e74a60db35dc345e9cc6580d804342c97a514f`).
A single asset-precalculation run reports 2,634.206 ms for the development intro.
The executable is 149,056 bytes larger than the earlier recorded 1,151,600-byte
build; the added released physics paths and intervening changes have not yet
received a size comparison under the tiny profile. These measurements do not
establish the requested distribution-size or runtime-performance outcome.
