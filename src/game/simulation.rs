//! Original OnTick phase order, sharing scene actors, collision and events.
use super::{
    event_runtime,
    input::Input,
    monster_ai, monster_magnet,
    player::{Motion, Sound},
    shot::Origin,
    splash::Targets,
    weapon, zones,
};
use crate::{
    collision::world::World,
    kx::Document,
    random::Random,
    scene::exec::{ActorContext, Execution},
};
use std::{cell::RefCell, rc::Rc};
pub struct Audio {
    pub sound: Sound,
    pub position: [f32; 4],
}
pub struct Listener {
    pub position: [f32; 4],
    pub velocity: [f32; 4],
    pub up: [f32; 4],
    pub forward: [f32; 4],
}
#[derive(Default)]
pub struct Tick {
    pub ran: bool,
    pub quit: bool,
    pub options_changed: bool,
    pub continued: bool,
    pub sounds: Vec<Audio>,
    pub listeners: Vec<Listener>,
}
pub struct Simulation {
    pub released_actors: Option<super::beta_monster::frame::ActorSimulation>,
    pub world: Rc<RefCell<World>>,
    pub motion: Motion,
    pub input: Input,
    pub zones: zones::Runtime,
    pub events: event_runtime::Runtime,
    pub random: Random,
    pub tick_count: i32,
    pub player_cell: Option<usize>,
    was_options: bool,
}
impl Simulation {
    fn released_world(
        world: &World,
        layout: crate::kx::Layout,
    ) -> Option<super::beta_monster::frame::ActorSimulation> {
        (layout == crate::kx::Layout::Beta2004).then(|| {
            super::beta_monster::frame::ActorSimulation::new(
                super::beta_monster::static_world::StaticWorld::new(
                    world
                        .entries
                        .iter()
                        .map(|entry| (entry.cell.vertices, entry.cell.mode)),
                ),
            )
        })
    }
    pub fn new(
        world: World,
        execution: &mut Execution,
        position: [f32; 4],
        now: i32,
        mouse: [i32; 2],
        random: Random,
    ) -> Result<Self, &'static str> {
        let mut motion = Motion::spawn_for_layout(&world, position, 0., execution.document_layout)?;
        motion.matrix = crate::mesh::identity();
        let player_cell = (motion.collider.cell != usize::MAX).then_some(motion.collider.cell);
        let zones = zones::Runtime::new_for_layout(&world, execution.document_layout);
        let events = event_runtime::Runtime::new(&world, execution);
        let released_actors = Self::released_world(&world, execution.document_layout);
        Ok(Self {
            released_actors,
            world: Rc::new(RefCell::new(world)),
            motion,
            input: Input::new(now, mouse),
            zones,
            events,
            random,
            tick_count: 0,
            player_cell,
            was_options: false,
        })
    }
    pub fn bind_scene(&self, execution: &mut Execution, time_reset: bool) {
        execution.observer_sector = self.player_cell.and_then(|cell| {
            self.world
                .borrow()
                .entries
                .get(cell)
                .and_then(|entry| entry.sector)
        });
        execution.actors = Some(ActorContext {
            world: Rc::clone(&self.world),
            player_position: self.motion.collider.position,
            released_player_cell: self
                .motion
                .released_player
                .as_ref()
                .and_then(|player| player.physics[0].0.particles[0].cell),
            player_hit: Some(Rc::clone(&self.motion.hit_cell)),
            time_reset,
        });
    }
    pub fn prepare_scene(
        &mut self,
        execution: &mut Execution,
        beat: i32,
        milliseconds: i32,
        aspect: f32,
        clear_instances: bool,
    ) -> Result<(), &'static str> {
        execution.init_frame(beat, milliseconds, clear_instances);
        execution.add_document_events();
        self.register_events(execution)?;
        self.bind_scene(execution, execution.time_reset);
        execution.game_camera.space = self.motion.camera(false);
        execution.game_camera.zoom_y = aspect;
        if execution.document_layout == crate::kx::Layout::Beta2004
            && let Some(pool) = execution.overlay.as_mut()
        {
            pool.reset();
        }
        Ok(())
    }
    pub fn execute_scene(
        &mut self,
        execution: &mut Execution,
        doc: &Document<'_>,
        root: usize,
    ) -> Result<(), crate::scene::Error> {
        execution.frame_with_random(doc, root, &mut self.random)
    }
    pub fn register_events(&mut self, execution: &mut Execution) -> Result<(), &'static str> {
        self.events.add_events(execution, &self.motion, &self.zones)
    }
    pub fn key(
        &mut self,
        execution: &mut Execution,
        key: u32,
        mouse: [i32; 2],
    ) -> Result<super::input::KeyResult, &'static str> {
        let result = self.input.key(
            key,
            mouse,
            &mut execution.game,
            &mut self.motion,
            &self.world.borrow(),
        )?;
        if result.placed {
            self.player_cell =
                (self.motion.collider.cell != usize::MAX).then_some(self.motion.collider.cell);
        }
        Ok(result)
    }
    /// Original ResetRoot after the selected scene has been assembled. Keep
    /// the shared world handle, player hit cell, input clock and weapon event.
    pub fn reset_root(
        &mut self,
        world: World,
        execution: &mut Execution,
        mouse: [i32; 2],
        first_time: bool,
    ) -> Result<(), &'static str> {
        self.events
            .replace_world(&world, execution, &mut self.motion);
        self.zones = zones::Runtime::new_for_layout(&world, execution.document_layout);
        self.released_actors = Self::released_world(&world, execution.document_layout);
        self.motion.document_layout = execution.document_layout;
        *self.world.borrow_mut() = world;
        self.restart(execution, mouse)?;
        if first_time {
            execution.switches[1] = 8;
        }
        self.bind_scene(execution, execution.time_reset);
        Ok(())
    }
    pub fn select_root(
        &mut self,
        execution: &mut Execution,
        mouse: [i32; 2],
    ) -> Result<super::root::Selection, &'static str> {
        if execution.document_layout == crate::kx::Layout::Beta2004 {
            return Ok(super::root::Selection {
                index: 0,
                quit: false,
                restart: false,
            });
        }
        let selection = super::root::select(&mut execution.switches);
        if selection.restart {
            self.restart(execution, mouse)?;
        }
        Ok(selection)
    }
    pub fn restart(
        &mut self,
        execution: &mut Execution,
        mouse: [i32; 2],
    ) -> Result<(), &'static str> {
        self.events.restart(
            execution,
            &mut self.motion,
            &self.world.borrow(),
            &mut self.zones,
        )?;
        self.player_cell =
            (self.motion.collider.cell != usize::MAX).then_some(self.motion.collider.cell);
        self.input.placement_mouse(mouse);
        Ok(())
    }
    pub fn tick(
        &mut self,
        execution: &mut Execution,
        doc: &Document<'_>,
        slices: i32,
        now: i32,
        mouse: [i32; 2],
    ) -> Result<Tick, &'static str> {
        let mut result = Tick::default();
        let mode = execution.switches[1];
        let options = mode == 9 || mode == 1 && execution.switches[16] == 1;
        result.options_changed = self.was_options && !options;
        self.was_options = options;
        if execution.document_layout == crate::kx::Layout::Beta2004 {
            use super::beta_player::{TickDispatch, dispatch_tick};
            let particle_cell = self
                .motion
                .released_player
                .as_ref()
                .is_some_and(|player| player.physics[0].0.particles[0].cell.is_some());
            match dispatch_tick(
                self.player_cell.is_some(),
                particle_cell,
                mode,
                &mut self.tick_count,
                slices,
            ) {
                TickDispatch::Idle => return Ok(result),
                TickDispatch::Restart => {
                    self.restart(execution, mouse)?;
                    return Ok(result);
                }
                TickDispatch::Quit => {
                    result.quit = true;
                    return Ok(result);
                }
                TickDispatch::Run => {}
            }
        } else {
            if self.player_cell.is_none() || mode != 0 {
                return Ok(result);
            }
            self.tick_count = self.tick_count.wrapping_add(slices);
        }
        result.ran = true;
        let elapsed = self.input.look(
            now,
            mouse,
            &execution.game,
            &mut self.motion,
            &execution.switches,
        );
        execution
            .game
            .weapon
            .animate(slices, self.motion.accel_forward);
        if let Some(sound) = weapon::step(&mut self.motion, slices) {
            result.sounds.push(Audio {
                sound,
                position: self.motion.collider.position,
            });
        }
        let mut world = self.world.borrow_mut();
        if let Some(sound) = execution.game.apply_splash(
            &mut execution.splash,
            Targets {
                player: self.motion.collider.position,
            },
            &mut world,
            &execution.dynamic_cells,
            &mut execution.switches,
        )? {
            result.sounds.push(Audio {
                sound,
                position: self.motion.collider.position,
            });
        }
        let (fire, sound) = execution.game.weapon.fire(
            &mut execution.game.inventory,
            &execution.game.parameters,
            &execution.game.events,
            slices,
        )?;
        if let Some(sound) = sound {
            result.sounds.push(Audio {
                sound,
                position: self.motion.collider.position,
            });
        }
        if let Some(fire) = fire {
            self.events.fire(
                execution,
                fire.weapon,
                Origin::Player {
                    position: self.motion.collider.position,
                    look: self.motion.look,
                    direction: self.motion.direction,
                    last_camera_y: self.motion.last_camera_y,
                    camera_offset: execution.game.parameters.camera_offset,
                    cell: self.player_cell.unwrap(),
                },
                &mut self.random,
            )?;
        }
        let actors: Vec<_> = execution.game.monsters.active().collect();
        for slice in 0..slices {
            let sound_position = self.motion.collider.position;
            let (sound, mut velocity) = if let Some(released) = self.released_actors.as_mut() {
                let player = self
                    .motion
                    .released_player
                    .as_ref()
                    .ok_or("released player body missing")?;
                let main = &player.physics[0].0.particles[0];
                let scene = execution
                    .dynamic_cells
                    .iter()
                    .map(|cell| {
                        super::beta_monster::scene_cells::SceneCell::capture(
                            Rc::clone(cell),
                            Rc::ptr_eq(cell, &self.motion.hit_cell),
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let clearance = released.player_clearance_with_cells(
                    main.vectors[0],
                    main.cell,
                    execution.game.parameters.gravity,
                    execution.game.parameters.stair_height,
                    &scene,
                )?;
                (
                    self.motion
                        .prepare_released(&execution.game.parameters, slices, clearance)?,
                    [0.; 4],
                )
            } else {
                self.motion.slice_with_velocity(
                    &execution.game.parameters,
                    &mut world,
                    slices,
                    &execution.dynamic_cells,
                )?
            };
            if let Some(sound) = sound {
                result.sounds.push(Audio {
                    sound,
                    position: sound_position,
                });
            }
            self.player_cell =
                (self.motion.collider.cell != usize::MAX).then_some(self.motion.collider.cell);
            let fade = (slice as f32 + 1.) / slices as f32;
            if self.released_actors.is_none() {
                execution.physics(fade);
            }
            let parameters = &execution.game.parameters;
            let mut released_settings = super::beta_monster::frame::Settings {
                physics: super::beta_monster::physics::Settings {
                    gravity: super::beta_monster::physics::PARTICLE_GRAVITY,
                    gravity_player: parameters.gravity,
                    damping_player_air: parameters.damp_air,
                    damping_player_ground: parameters.damp_ground,
                    fly_mode: 0,
                    on_ground: self.motion.on_ground,
                },
                ai: super::beta_monster::ai::Settings {
                    gravity: parameters.gravity,
                    height: parameters.stair_height,
                    height_gain: parameters.damp_height,
                },
                player: self.motion.collider.position,
                player_cell: self
                    .player_cell
                    .and_then(|source| world.adds.iter().position(|&id| id == source)),
                first_particle: 5,
            };
            let mut released_frame = if let Some(released) = self.released_actors.as_mut() {
                let player = self
                    .motion
                    .released_player
                    .as_mut()
                    .ok_or("released player body missing")?;
                let scene = execution
                    .dynamic_cells
                    .iter()
                    .map(|cell| {
                        let mut captured = super::beta_monster::scene_cells::SceneCell::capture(
                            Rc::clone(cell),
                            Rc::ptr_eq(cell, &self.motion.hit_cell),
                        )?;
                        captured.propose(fade)?;
                        Ok(captured)
                    })
                    .collect::<Result<Vec<_>, &'static str>>()?;
                player.physics[0].1.skip_cell =
                    scene.iter().position(|cell| cell.player).and_then(|index| {
                        execution.released_order.iter().position(|slot| {
                            *slot == super::beta_monster::registration::DynamicSlot::Scene(index)
                        })
                    });
                let first_point = execution
                    .game
                    .released_monsters
                    .active_ids()
                    .iter()
                    .copied()
                    .max()
                    .unwrap_or(0) as usize;
                let first_point = first_point
                    .checked_mul(5)
                    .and_then(|n| released_settings.first_particle.checked_add(n))
                    .ok_or("released point particle identity overflow")?;
                let projectiles = super::beta_monster::projectile::prepare_projectiles(
                    &execution.released_points,
                    &execution.released_boxes,
                    first_point,
                    player.physics[0].1.skip_cell,
                    &execution.events,
                    &mut released.hits,
                )?;
                let mut operators = [[0; 8]; 2];
                for (target, source) in operators.iter_mut().zip(&execution.game.events.explode) {
                    for (target, source) in target.iter_mut().zip(source) {
                        *target = source
                            .map(|id| {
                                u32::try_from(id)
                                    .ok()
                                    .and_then(|id| id.checked_add(1))
                                    .ok_or("released impact operator identity overflow")
                            })
                            .transpose()?
                            .unwrap_or(0);
                    }
                }
                released.hits.operators = operators;
                for (target, &source) in released.hits.flags[8..16]
                    .iter_mut()
                    .zip(&execution.game.parameters.weapon_flags)
                {
                    *target = source as u32;
                }
                released.hits.time = self.tick_count;
                let accepted = released.physics_frame(
                    super::beta_monster::frame::FrameInput {
                        registry: &execution.game.released_monsters,
                        colliders: &execution.released_colliders,
                        scene,
                        dynamic_order: Some(&execution.released_order),
                        points: &projectiles.points,
                        boxes: &projectiles.boxes,
                        physics_order: Some(&execution.released_physics_order),
                    },
                    &mut player.physics,
                    &released_settings,
                )?;
                execution.sync_released_hit_events(&released.hits)?;
                velocity = self.motion.publish_released(&world)?;
                self.player_cell =
                    (self.motion.collider.cell != usize::MAX).then_some(self.motion.collider.cell);
                Some(accepted)
            } else {
                None
            };

            if let Some(released) = self.released_actors.as_mut() {
                released.publish_zone_interactions(&mut world)?;
            }
            let resets = self.zones.tick(
                &mut execution.game,
                &mut self.motion,
                &mut world,
                &execution.dynamic_cells,
                &mut execution.switches,
            )?;
            self.events.reset_zone_events(execution, &resets)?;
            self.motion.update_camera(&execution.game.parameters);
            for value in &mut velocity[..3] {
                *value *= 1000. / elapsed as f32;
            }
            result.listeners.push(Listener {
                position: self.motion.camera_position,
                velocity,
                up: self.motion.matrix[1],
                forward: self.motion.matrix[2],
            });

            if let (Some(released), Some(accepted)) =
                (self.released_actors.as_mut(), released_frame.as_mut())
            {
                for sound in released.apply_weapon_hits(
                    accepted,
                    &execution.game.released_monsters,
                    &mut execution.game.inventory,
                    &execution.game.parameters.weapon_hits,
                    &mut execution.switches,
                )? {
                    result.sounds.push(Audio {
                        sound,
                        position: self.motion.collider.position,
                    });
                }
                released_settings.player = self.motion.collider.position;
                released_settings.player_cell = self
                    .player_cell
                    .and_then(|source| world.adds.iter().position(|&id| id == source));
                for sound in released.think(
                    &execution.game.released_monsters,
                    accepted,
                    &mut execution.game.inventory,
                    &released_settings,
                )? {
                    result.sounds.push(Audio {
                        sound,
                        position: self.motion.collider.position,
                    });
                }
                execution.publish_released_actor_events(doc, &mut released.hits.pending)?;
            }
            monster_magnet::apply(&actors, &mut self.random);
            let mut previous = false;
            let mut machine = false;
            for actor in &actors {
                let spline = actor
                    .borrow()
                    .spline
                    .map(|id| doc.splines.get(id).ok_or("monster spline out of range"))
                    .transpose()?;
                let actions = actor.borrow_mut().think(monster_ai::Context {
                    world: &mut world,
                    dynamic: &execution.dynamic_cells,
                    player_hit: Some(&self.motion.hit_cell),
                    player: monster_ai::Player {
                        position: self.motion.collider.position,
                        collider_position: self.motion.collider.position,
                        cell: Some(self.motion.collider.cell),
                        visibility_cell: self.player_cell,
                    },
                    parameters: &execution.game.parameters,
                    switches: &mut execution.switches,
                    spline,
                    time: execution.current_time,
                    time_reset: execution.actors.as_ref().is_some_and(|a| a.time_reset),
                    machine_alive: machine,
                    previous_alive: previous,
                })?;
                if let Some(hits) = actions.player_hits
                    && let Some(sound) = execution.game.inventory.hit(hits)
                {
                    result.sounds.push(Audio {
                        sound,
                        position: self.motion.collider.position,
                    });
                }
                if let Some(fire) = actions.fire {
                    self.events
                        .fire(execution, fire.weapon, fire.origin, &mut self.random)?;
                }
                let actor = actor.borrow();
                previous = actor.health.life > 0;
                if actor.parameters.flags & 4 != 0 {
                    machine = previous;
                    previous = false;
                }
            }
            for sound in self
                .events
                .advance_shots(execution, &self.motion, &mut world)?
            {
                result.sounds.push(Audio {
                    sound,
                    position: self.motion.collider.position,
                });
            }
            execution.game.weapon.use_key = false;
            if self.player_cell.is_none_or(|id| {
                world.entries[id]
                    .cell
                    .outside_mask(self.motion.collider.position, 0.)
                    != 0
            }) && let Some(id) = world.find_cell(self.motion.collider.position)
            {
                self.player_cell = Some(id);
            }
        }
        execution.finish_collision_tick();
        if execution.game.inventory.amounts[0] == 0 && execution.switches[1] == 0 {
            execution
                .game
                .continue_game(&mut self.motion, &world, &execution.switches)?;
            self.player_cell =
                (self.motion.collider.cell != usize::MAX).then_some(self.motion.collider.cell);
            self.input.placement_mouse(mouse);
            result.continued = true;
        }
        Ok(result)
    }
}
