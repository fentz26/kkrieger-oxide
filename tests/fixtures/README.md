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

`prepared-mesh.bin` records 512 original EngMesh preparation jobs, covering
static triangles, shadow planes/silhouettes, sprites and thick lines, plus
vertex packing and part bounds. The source leaves final part endpoints
unwritten and never reads them; the oracle initializes these unused entries
to -1 and Rust represents them as None. Regenerate with
`python3 tools/verify_prepared_mesh.py`.

`lighting-lookup.bin` records all 229,376 RGBA bytes from the original 64²
cube normalizer and 32³ attenuation generators, with GPU allocations replaced
by memory buffers. Regenerate with `python3 tools/verify_lighting_lookup.py`.

`material-uniforms.bin` records 128 original Material11 Set calls and their
active constant registers. Unwritten source registers are excluded from fidelity
comparisons. The native matrix adapter replaces adjacent-member output indexing
with explicit component access while retaining arithmetic order. Regenerate with
`python3 tools/verify_material_uniforms.py`.

`texture-upload.bin` records 256 original player UpdateTexture calls, including
unsigned RGBA, signed Q8W8V8U8, mip limits, rectangular/NPOT inputs, threshold
filtering and inverted mip filtering. D3D allocations and locks use memory-only
stubs. Regenerate with `python3 tools/verify_texture_upload.py`.

`shadow.bin` records 128 original UpdateShadowCacheJob results and the paired
homogeneous extrusion vertices. Classification retains the original global
face table across cases, including entries beyond the current plane count.
Regenerate with `python3 tools/verify_shadow.py`.

`visibility.bin` contains 128 original C++ portal graphs with two successive
views, including near clipping, cycles, costs and retained sector bounds.
Regenerate with `python3 tools/verify_visibility.py`.

`delay.bin` records 256 sequences of 80 original Misc_Delay calls, including
hysteresis, all variable routes, switch inputs, phase transitions, nonpositive
durations and large/negative time steps. Regenerate with
`python3 tools/verify_delay.py`. Child execution is stubbed in this oracle;
it proves the persistent envelope and variable writes, not instance allocation.

`collision.bin` records the original KKriegerCell vertex constructor for 256
warped, inverted, flat, repeated-corner and ordinary boxes, including face planes,
overlap expansion, bounds and center. Regenerate with
`python3 tools/verify_collision.py`.

`collision-dynamic.bin` records 128 cells through original PrepareDynamic,
ApplyMatrix and the dynamic-cell Physic loop, including interpolated, extrapolated
and sheared transforms. It checks all plane/vertex and matrix fields; regenerate
with `python3 tools/verify_collision.py`.

`collision-queries.bin` records 4,096 original OutsideMask, RejectPlane,
approxDistance, IntersectIn and IntersectOut calls. It includes radii, parallel
and zero directions, tiny determinants, warped cells and degenerate cells.
Regenerate with `python3 tools/verify_collision.py`.

`collision-connectivity.bin` records 128 original heap-sort and bounding-box
connection graphs, including empty graphs, equal keys and all cell modes.
It checks sorted order and ordered add/sub/zone adjacency. Collision-face
generation is excluded from this oracle. Regenerate with
`python3 tools/verify_connectivity.py`.

`collision-faces.bin` records 128 complete original CreateCollisionFaces outputs,
including recursive clipping and linked-list order. The native layout preserves
Planes[12] followed by Center because SplitCollisionFace reads one extra plane.
Regenerate with `python3 tools/verify_collision_faces.py`.

`collision-world.bin` records 64 original SetSceneR/AddMesh traversals with
shared children, nested sectors, repeated/negative counts and attached events.
Checks include cell creation order, transforms, scene identity, logic and event
placement. Regenerate with `python3 tools/verify_collision_world.py`; this also
regenerates the face fixture imported by its native harness.

`collision-ray.bin` records 1,280 original world ray queries through connected
additive cells, static/dynamic obstacles and persistent zone flags. It verifies
the 16-cell traversal cap, all result fields and dynamic-cell pointer identity.
Regenerate with `python3 tools/verify_collision_ray.py`; its native harness also
regenerates the imported world and face fixtures.

`collision-sphere.bin` records 4,096 original sphere/face/edge checks and soft
responses. It includes output writes on false results, degenerate polygons and
coincident contacts. Only initialized nearest-point xyz components are recorded;
the source nearest-point w is not read by these algorithms. Regenerate with
`python3 tools/verify_collision_sphere.py`; imported world/face fixtures are also
regenerated by the native harness.

`collision-movement.bin` records 1,280 complete original MoveCollider calls
through generated faces and dynamic geometry, including player/monster exclusions,
active-cell changes, radius preservation and persistent zone flags. Regenerate
with `python3 tools/verify_collision_movement.py`; imported prerequisite fixtures
are regenerated as part of its native harness.

`game-parameters.bin` records 384 original parameter applications across initial,
live and reset calls. `game-events.bin` records 384 original registrations across
all four weapon event groups, default routing and null links. Regenerate both
with `python3 tools/verify_game_parameters.py`.

`player-motion.bin` records 768 original OnTick player-movement prefixes, ending
before each slice's Physic/Zones phase. It checks grounding, damping, integer
stair lift, slice-count air time, landing requests, movement history, hit-cell
matrices and zone masks. Regenerate with `python3 tools/verify_player_motion.py`;
imported collision fixtures are regenerated by its native harness.

`player-actions.bin` records 1,024 original jump/damage pairs, including jump
gating, persistent life/armor, impulse/timer changes and ordered sound requests.
The original sound helpers run against a recording backend to verify volume and
3D distance ranges. Regenerate with `python3 tools/verify_player_actions.py`.

`weapon-state.bin` records 1,024 original weapon frames: animation, switch
decisions, footsteps, empty-ammo sounds, firing requests, ammo and cooldowns.
Projectile creation and event lifecycle are recorded as calls, not emulated.
Regenerate with `python3 tools/verify_weapon_state.py`.

`shot-spawn.bin` records all eight original ShotInfo definitions and 128 FireShot
initializations for player and monster origins. It checks matrices, event fields,
old positions, cube tensors and subsequent random-stream state, including the
disabled mode. Regenerate with `python3 tools/verify_shot_spawn.py`; imported
collision/player fixtures are also regenerated by its native harness.

`shot-ai.bin` records 64 original projectile trajectories through real collision
traversal. It checks point/cube movement, rotation, bounces, player and monster
damage requests, explosion matrices, null event links and zone masks. Damage and
dynamic-event helpers record calls; consuming those requests is a separate phase.
Regenerate with `python3 tools/verify_shot_ai.py`.

`monster-damage.bin` records 1,024 original monster hits, including dead-monster
guards, activation, unchanged armor, disabled kill switches and byte wrapping.
Regenerate with `python3 tools/verify_monster_damage.py`.

`splash.bin` records 64 complete original splash phases with real visibility rays
and damage methods. `splash-trigger.bin` records 512 original per-instance trigger
executions. Checks cover falloff, blocked/outside-volume origins, zero-damage
activation, halved player damage, kill switches and rising-edge state. Regenerate
with `python3 tools/verify_splash.py`; imported collision fixtures are regenerated.

`checkpoints.bin` records 64 original checkpoint registration, Continue, RespawnAt
and SetPlayer sequences. It checks checkpoint priority, retained activation,
living-monster reset, minimum ammunition, hit-cell matrices, preserved movement
timers and camera placement when collision falls back to the first volume.
Regenerate with `python3 tools/verify_checkpoints.py`; imported collision/player
fixtures are also regenerated.

`zones.bin` records 3,328 original Zones phases over 832 three-zone sequences,
covering all trigger/action nibbles, condition gates, ordered switch effects,
collectable aliases, weapon selection, jump impulses, checkpoint flags and
respawn countdown boundaries. Native event stubs record chain deletion and
pointer clearing; the Rust test consumes ordered reset requests against the same
stub state. Regenerate with `python3 tools/verify_zones.py`.

`events.bin` records 768 native KEnvironment event schedules and 128 original
KOp::ExecEvent contexts. Scheduling checks activation intervals, equal start/end,
newest-first operator lists, ClearInstanceMem and swap-removal expiry order.
Scoped execution checks all eight event variables, interval clamping, inherited
splines, monster/current-event context restoration and Stop. Instance and stack
stubs record calls; affine multiplication is covered separately by scene tests.
Regenerate with `python3 tools/verify_events.py`.

`trigger.bin` records 2,048 original Misc_Trigger executions with persistent
instance memory. It covers inverted/repeating/parent-position flags, invalid
links, first-use state, delayed/fractional timing, large beat values, negative
intervals, generated event payloads and unconditional child execution. The
original handler ignores the supplied color and spline arguments. Regenerate
with `python3 tools/verify_trigger.py`; it also regenerates `events.bin`.

`game-event-registration.bin` records 384 original KKriegerGame::AddEvents phases with the
native event scheduler. It covers shot selection/swap removal, mode-dependent
matrix copying, zone event selection, weapon switching and optics variables.
Native matrix tokens test assignment rather than matrix arithmetic; scene tests
cover transforms separately. Regenerate with `python3 tools/verify_game_events.py`;
`events.bin` is also regenerated.

`restart-camera.bin` records 64 native Restart calls and 256 camera updates.
Restart/GetCamera/DoRespawn use unchanged source functions; the camera update
uses the unchanged four-statement OnTick fragment. Checks cover collision
fallback, gameplay-switch clearing, preserved inventory/timers, dead-monster
reset, zone cleanup, height damping and both camera-space branches. Regenerate
with `python3 tools/verify_restart_camera.py`; imported collision/player/checkpoint
fixtures are also regenerated.

`input-keys.bin` records 3,072 unchanged player-build OnKey calls, including
release masks, ignored qualifiers, movement, jump/hurt sounds, use/fire, fly mode,
weapon mapping and cheats. Respawn placement is a recording stub; the actual
placement and camera behavior are covered by checkpoint fixtures. Regenerate
with `python3 tools/verify_input.py` (also regenerates player-actions.bin).

`input-look.bin` records 1,024 executions of the unchanged OnTick mouse prefix
with original Euler conversion, covering inversion, speed, clamping, fly mode,
negative clock deltas and the 999/1000/1001 ms cutoff. Regenerate with
`python3 tools/verify_input_look.py`; imported collision/player fixtures also run.

`monster-walk.bin` records 2,560 unchanged KKMonsterWalk calls with native
collision rays and original spline evaluation. It covers all five original
styles, all orientation/step flags, null and real splines, gait transitions,
reset/time reversal, absent volumes, dynamic obstacles and deviation handling.
Every persistent field, foot-cell identity and output matrix is compared; NaNs
are compared by classification. Regenerate with `python3 tools/verify_monster_walk.py`;
imported collision/player fixtures are regenerated too.

`monster-actor.bin` records 120 runs of the unchanged initialization branch from
Exec_KKrieger_Monster, with original collision, dynamic-cell preparation, walking
and spline evaluation. It covers all five styles, outside-volume spawns, retained
attack timers/magnet force on respawn and monster overrides. The native scaffold
asserts the original 14-word parameter-block layout: its 16-word copy overwrites
the first two event-matrix floats. Scene registration/rendering after that branch
is excluded. Regenerate with `python3 tools/verify_monster_actor.py`; imported
collision/player/walking fixtures are regenerated too.

`monster-ai.bin` records 1,280 unchanged MonsterAI calls with real collision,
sphere movement, walking and monster damage. Player Hit and FireShot record
requests, including the firing position before movement. Cases cover spawn
switches/dependencies, visibility, melee/ranged attacks, immobility, missing
player/monster cells, death, kill switches and render-event fields. Full actor,
hit-cell and walk state is checked after each call. Regenerate with
`python3 tools/verify_monster_ai.py`; imported actor/walk/collision fixtures also run.
