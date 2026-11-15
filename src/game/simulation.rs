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
    pub options_changed: bool,
    pub continued: bool,
    pub sounds: Vec<Audio>,
    pub listeners: Vec<Listener>,
}
pub struct Simulation {
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
    pub fn new(
        world: World,
        execution: &mut Execution,
        position: [f32; 4],
        now: i32,
        mouse: [i32; 2],
        random: Random,
    ) -> Result<Self, &'static str> {
        let mut motion = Motion::spawn(&world, position, 0.)?;
        motion.matrix = crate::mesh::identity();
        let player_cell = (motion.collider.cell != usize::MAX).then_some(motion.collider.cell);
        let zones = zones::Runtime::new(&world);
        let events = event_runtime::Runtime::new(&world, execution);
        Ok(Self {
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
        execution.actors = Some(ActorContext {
            world: Rc::clone(&self.world),
            player_position: self.motion.collider.position,
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
        Ok(())
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
    pub fn select_root(
        &mut self,
        execution: &mut Execution,
        mouse: [i32; 2],
    ) -> Result<super::root::Selection, &'static str> {
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
        if self.player_cell.is_none() || mode != 0 {
            return Ok(result);
        }
        result.ran = true;
        self.tick_count = self.tick_count.wrapping_add(slices);
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
            let (sound, mut velocity) = self.motion.slice_with_velocity(
                &execution.game.parameters,
                &mut world,
                slices,
                &execution.dynamic_cells,
            )?;
            if let Some(sound) = sound {
                result.sounds.push(Audio {
                    sound,
                    position: sound_position,
                });
            }
            self.player_cell =
                (self.motion.collider.cell != usize::MAX).then_some(self.motion.collider.cell);
            execution.physics((slice as f32 + 1.) / slices as f32);
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
