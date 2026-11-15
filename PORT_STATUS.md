# Full-game rewrite

Required outcome: the entire original .kkrieger game implemented in Rust,
preserving its content and gameplay, with no invented game content. Runtime
performance and distribution size must be measured and optimized. The current
prototype does not satisfy that outcome. The goal remains active.

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
