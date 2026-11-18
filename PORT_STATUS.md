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
  measured macOS ARM64 release executable after packing was 1,151,600 bytes,
  66,048 bytes smaller than the preceding build. Later changes require remeasurement.

The current local audit passes 343 tests across 147 test binaries, with 28
ignored checks. `cargo fmt --all --check` and Clippy across all targets pass.
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
