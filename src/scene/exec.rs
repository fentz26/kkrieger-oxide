//! Scene Exec handlers emit the original engine jobs. Unported gameplay is an
//! error, so an incomplete frame can never masquerade as a faithful game frame.
use super::{
    Error, delay,
    instance::{Memory, Scope},
};
use crate::collision::DynamicCell;
use crate::{
    animation::Vm,
    kx::Document,
    mesh::{Matrix, identity, srt_matrix},
    mesh_graph,
    visibility::{self, Portal, Rect},
};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Debug)]
pub enum Job {
    Paint {
        operator: usize,
        matrix: Matrix,
        time: f32,
        pass_adjust: i32,
    },
    Light {
        operator: usize,
        position: [f32; 4],
        flags: u32,
        color: u32,
        amplify: f32,
        range: f32,
        clip: Rect,
    },
    Portal {
        operator: usize,
        portal: Portal,
    },
    Sector {
        operator: usize,
        input: usize,
        matrix: Matrix,
    },
}

#[derive(Clone)]
pub struct ActorContext {
    pub world: Rc<RefCell<crate::collision::world::World>>,
    pub player_position: [f32; 4],
    pub player_hit: Option<Rc<RefCell<DynamicCell>>>,
    pub time_reset: bool,
}

pub struct Execution {
    pub vm: Vm,
    pub game_camera: super::camera::Environment,
    pub current_camera: super::camera::Environment,
    pub pass_adjust: i32,
    pub aspect: f32,
    pub markers: [Matrix; 32],
    pub jobs: Vec<Job>,
    pub overlay: Option<super::overlay::Pool>,
    pub overlay_draws: Vec<super::overlay::Draw>,
    pub dynamic_cells: Vec<Rc<RefCell<DynamicCell>>>,
    pub event_monster: Option<usize>,
    pub event_spline: Option<usize>,
    pub current_event: Option<super::event::Id>,
    pub events: super::event::Scheduler,
    pub switches: [u8; 256],
    pub game: crate::game::State,
    pub splash: crate::game::splash::Splash,
    pub actors: Option<ActorContext>,
    pub next_monster_override: Option<crate::game::monster_actor::Override>,
    pub current_time: i32,
    pub last_time: i32,
    pub time_delta: i32,
    pub time_slices: i32,
    pub time_jitter: i32,
    pub time_reset: bool,
    pub clear_instances: bool,
    instances: Memory,
    parameters: Vec<Vec<u32>>,
    current_clip: Rect,
    visibility_history: Vec<Rect>,
}
impl Execution {
    pub fn new(doc: &Document<'_>) -> Self {
        Self {
            vm: Vm::new(),
            game_camera: super::camera::Environment::default(),
            current_camera: super::camera::Environment::default(),
            pass_adjust: 0,
            aspect: 1.,
            markers: [identity(); 32],
            jobs: Vec::new(),
            overlay: None,
            overlay_draws: Vec::new(),
            dynamic_cells: Vec::new(),
            event_monster: None,
            event_spline: None,
            current_event: None,
            events: super::event::Scheduler::from_document(doc),
            switches: [0; 256],
            game: crate::game::State::default(),
            splash: crate::game::splash::Splash::default(),
            actors: None,
            next_monster_override: None,
            current_time: 0,
            last_time: 0,
            time_delta: 0,
            time_slices: 0,
            time_jitter: 0,
            time_reset: true,
            clear_instances: false,
            instances: Memory::default(),
            parameters: doc.operators.iter().map(|o| o.parameters.clone()).collect(),
            current_clip: Rect::UNIT,
            visibility_history: Vec::new(),
        }
    }
    pub fn new_player(doc: &Document<'_>) -> Self {
        let mut execution = Self::new(doc);
        execution.game = crate::game::State::player_build();
        execution.switches = crate::game::root::switches();
        execution
    }
    /// Original InitView boundary: discard dynamic and default-owned instance
    /// chains while retaining document events and per-operator scene memory.
    pub fn reset_view(&mut self) {
        for id in self.events.reset_view() {
            self.reset_event_instances(id);
        }
        self.clear_scope(Scope::Default);
        self.instances.enter(Scope::Default);
        self.clear_instances = true;
        self.aspect = 1.;
        self.current_time = 0;
        self.last_time = 0;
        self.time_delta = 0;
        self.time_slices = 0;
        self.time_jitter = 0;
        self.time_reset = true;
        self.markers.fill(identity());
        self.game.monsters.begin_frame();
    }
    /// Original InitFrame CPU state. Document and game event registration follow
    /// this call, after the previous frame's simulation has already run.
    pub fn init_frame(&mut self, beat: i32, milliseconds: i32, clear_instances: bool) {
        self.vm.begin_frame(beat);
        self.game_camera = super::camera::Environment::default();
        self.current_camera = super::camera::Environment::default();
        self.event_spline = None;
        self.event_monster = None;
        self.current_event = None;
        self.last_time = self.current_time;
        self.current_time = milliseconds;
        self.time_delta = self.current_time.wrapping_sub(self.last_time);
        self.time_reset = self.time_delta < 0;
        if self.time_reset {
            self.time_delta = 0;
            self.time_slices = 0;
            self.time_jitter = 0;
        } else {
            self.time_slices = (self.current_time / 10).wrapping_sub(self.last_time / 10);
            self.time_jitter = self.current_time % 10;
        }
        self.clear_instances = clear_instances;
        let resets = self.events.begin(beat, clear_instances);
        for id in resets {
            self.reset_event_instances(id);
        }
        self.instances.enter(Scope::Default);
        self.next_monster_override = None;
        if let Some(actors) = &mut self.actors {
            actors.time_reset = self.time_reset;
        }
    }
    pub fn add_event(&mut self, id: super::event::Id) -> bool {
        let active = self.events.add(id);
        if active && self.clear_instances {
            self.reset_event_instances(id);
        }
        active
    }
    pub fn add_document_events(&mut self) {
        for id in self.events.document.clone() {
            self.add_event(id);
        }
    }
    /// Event-only registration for callers that manage frame state separately.
    pub fn begin_events(&mut self, beat: i32, clear_instances: bool) {
        self.clear_instances = clear_instances;
        let resets = self.events.begin(beat, clear_instances);
        for id in resets {
            self.reset_event_instances(id);
        }
        self.add_document_events();
    }
    pub fn has_event_instances(&self, id: super::event::Id) -> bool {
        self.instances.contains(Scope::Event(id))
    }
    pub fn reset_event_instances(&mut self, id: super::event::Id) {
        self.clear_scope(Scope::Event(id));
    }
    fn clear_scope(&mut self, scope: Scope) {
        for monster in self.instances.clear(scope) {
            self.clear_scope(Scope::Event(monster.live));
            self.clear_scope(Scope::Event(monster.death));
            self.events.events.remove(&monster.live);
            self.events.events.remove(&monster.death);
            self.game.monsters.remove(monster.actor);
        }
    }
    pub fn finish_events(&mut self) {
        self.clear_instances = false;
        for id in self.events.finish() {
            self.reset_event_instances(id);
        }
    }
    pub fn execute_event(
        &mut self,
        doc: &Document<'_>,
        input: usize,
        id: super::event::Id,
        parent: Matrix,
    ) -> Result<(), Error> {
        let event = self.events.events.get(&id).cloned().ok_or(Error {
            operator: input,
            reason: "event missing",
        })?;
        let event = event.borrow().clone();
        let saved: [[f32; 4]; 8] = self.vm.variables[..8].try_into().unwrap();
        let old_monster = self.event_monster;
        let old_spline = self.event_spline;
        let old_event = self.current_event;
        self.vm.variables[..8].copy_from_slice(&event.variables(self.events.beat));
        self.event_monster = event.monster;
        if event.spline.is_some() {
            self.event_spline = event.spline;
        }
        self.current_event = Some(id);
        let cursor = self.instances.enter(Scope::Event(id));
        let result = self.visit(doc, input, multiply(event.matrix, parent));
        self.instances.restore(cursor);
        self.vm.variables[..8].copy_from_slice(&saved);
        self.event_monster = old_monster;
        self.event_spline = old_spline;
        self.current_event = old_event;
        result
    }
    pub fn frame(&mut self, doc: &Document<'_>, root: usize) -> Result<(), Error> {
        self.jobs.clear();
        self.overlay_draws.clear();
        self.dynamic_cells.clear();
        self.game.monsters.begin_frame();
        self.next_monster_override = None;
        if let Some(hit) = self.actors.as_ref().and_then(|a| a.player_hit.as_ref()) {
            self.dynamic_cells.push(Rc::clone(hit));
        }
        self.current_clip = Rect::UNIT;
        self.visibility_history.clear();
        self.instances.enter(Scope::Default);
        let result = self.visit(doc, root, identity());
        if result.is_err() {
            self.jobs.clear();
            self.overlay_draws.clear();
            self.dynamic_cells.clear();
            self.game.monsters.begin_frame();
        }
        result
    }
    /// Execute deferred sector contents in original newest-first order. Sector
    /// visibility is the enabled source branch. Remaining unported gameplay
    /// handlers report errors rather than dropping scene content.
    pub fn process_sectors(
        &mut self,
        doc: &Document<'_>,
        observer: Option<usize>,
        projection: Matrix,
    ) -> Result<(), Error> {
        use std::collections::BTreeMap;
        let sectors: Vec<_> = self
            .jobs
            .iter()
            .filter_map(|job| match job {
                Job::Sector {
                    operator,
                    input,
                    matrix,
                } => Some((*operator, *input, *matrix)),
                _ => None,
            })
            .collect();
        let ids: BTreeMap<_, _> = sectors
            .iter()
            .enumerate()
            .map(|(i, (id, _, _))| (*id, i))
            .collect();
        let result = (|| {
            if ids.len() != sectors.len() {
                return Err(Error {
                    operator: 0,
                    reason: "sector scene registered more than once",
                });
            }
            let fail = |reason| Error {
                operator: observer.unwrap_or(0),
                reason,
            };
            let index = |id: usize| {
                ids.get(&id)
                    .copied()
                    .ok_or_else(|| fail("portal scene has no registered sector"))
            };
            let mut portals = Vec::new();
            if observer.is_some() {
                for job in &self.jobs {
                    if let Job::Portal { portal, .. } = job {
                        let mut portal = portal.clone();
                        portal.sectors = [index(portal.sectors[0])?, index(portal.sectors[1])?];
                        portal.between = portal.between.map(index).transpose()?;
                        portals.push(portal);
                    }
                }
            }
            let observer = observer.map(index).transpose()?;
            let visible = visibility::compute_with_history(
                sectors.len(),
                &portals,
                observer,
                projection,
                &self.visibility_history,
            )
            .map_err(fail)?;
            self.visibility_history = visible.iter().map(|s| s.bounds).collect();
            for (i, &(_, input, matrix)) in sectors.iter().enumerate().rev() {
                if visible[i].execute {
                    self.current_clip = visible[i].bounds;
                    self.visit_scene(doc, input, matrix)?;
                }
            }
            Ok(())
        })();
        self.current_clip = Rect::UNIT;
        if result.is_err() {
            self.jobs.clear();
            self.overlay_draws.clear();
            self.dynamic_cells.clear();
            self.game.monsters.begin_frame();
            self.visibility_history.clear();
        }
        result
    }
    /// Original dynamic-cell phase; caller supplies the fixed-tick interpolation.
    pub fn physics(&mut self, fade: f32) {
        for cell in &self.dynamic_cells {
            cell.borrow_mut().physics(fade);
        }
    }

    /// Original OnTick collision endpoint update, after movement has set Blocked.
    pub fn finish_collision_tick(&mut self) {
        for cell in &self.dynamic_cells {
            let mut cell = cell.borrow_mut();
            if cell.blocked == 0 {
                cell.matrix0 = cell.matrix1;
            }
        }
    }

    fn visit_scene(
        &mut self,
        doc: &Document<'_>,
        root: usize,
        matrix: Matrix,
    ) -> Result<(), Error> {
        let saved = self.instances.enter(Scope::Scene(root));
        let result = self.visit(doc, root, matrix);
        self.instances.restore(saved);
        result
    }
    fn exec_inputs(
        &mut self,
        doc: &Document<'_>,
        root: usize,
        matrix: Matrix,
    ) -> Result<(), Error> {
        let op = &doc.operators[root];
        for &input in &op.inputs {
            self.visit(doc, input, matrix)?;
        }
        if doc.classes[op.class].convention & 0x1000_0000 == 0 {
            for &link in op.links.iter().flatten() {
                self.visit(doc, link, matrix)?;
            }
        }
        Ok(())
    }
    fn inputs(&mut self, doc: &Document<'_>, root: usize, matrix: Matrix) -> Result<(), Error> {
        for &input in &doc.operators[root].inputs {
            let class = doc.classes[doc.operators[input].class].id;
            if mesh_graph::implemented(class) || matches!(class, 0x61 | 0x63 | 0x64) {
                self.jobs.push(Job::Paint {
                    operator: input,
                    matrix,
                    time: self.vm.variables[0][0],
                    pass_adjust: self.pass_adjust,
                });
            } else {
                self.visit(doc, input, matrix)?;
            }
        }
        Ok(())
    }
    fn visit(&mut self, doc: &Document<'_>, root: usize, parent: Matrix) -> Result<(), Error> {
        let fail = |reason| Error {
            operator: root,
            reason,
        };
        let op = doc
            .operators
            .get(root)
            .ok_or_else(|| fail("execution root out of range"))?;
        let id = doc.classes[op.class].id;
        let p = self
            .parameters
            .get_mut(root)
            .ok_or_else(|| fail("execution document mismatch"))?;
        let eval = self
            .vm
            .execute(
                op,
                p,
                &doc.splines,
                self.event_spline.and_then(|i| doc.splines.get(i)),
            )
            .map_err(|_| fail("scene animation failed"))?;
        let p = p.clone();
        let f = |i| {
            p.get(i)
                .copied()
                .map(f32::from_bits)
                .ok_or_else(|| fail("truncated scene parameters"))
        };
        let srt = || -> Result<Matrix, Error> {
            let p = p
                .get(..9)
                .ok_or_else(|| fail("truncated scene transform"))?;
            Ok(srt_matrix(&std::array::from_fn(|i| f32::from_bits(p[i]))))
        };
        let result = (|| match id {
            0xe6 => {
                let params = p
                    .get(..5)
                    .ok_or_else(|| fail("truncated IPP color parameters"))?;
                let cached = self
                    .overlay
                    .as_ref()
                    .ok_or_else(|| fail("postprocessing target pool missing"))?
                    .find(Some(root));
                if let Some(cached) = cached {
                    self.overlay.as_mut().unwrap().last_output = Some(cached);
                } else {
                    self.exec_inputs(doc, root, parent)?;
                    if let Some(draw) = self
                        .overlay
                        .as_mut()
                        .ok_or_else(|| fail("postprocessing target pool missing"))?
                        .color_operator(
                            root,
                            params[0] as usize,
                            params[1],
                            params[2] as usize,
                            params[3],
                            params[4] as i32,
                        )
                        .map_err(fail)?
                    {
                        self.overlay_draws.push(draw);
                    }
                }
                Ok(())
            }
            0xe2 => {
                let params = p
                    .get(..4)
                    .ok_or_else(|| fail("truncated IPP copy parameters"))?;
                self.exec_inputs(doc, root, parent)?;
                let overlay = self
                    .overlay
                    .as_mut()
                    .ok_or_else(|| fail("postprocessing target pool missing"))?;
                if let Some(draw) = overlay
                    .copy_operator(
                        root,
                        params[0] as usize,
                        params[1],
                        f32::from_bits(params[2]),
                        params[3] as i32,
                    )
                    .map_err(fail)?
                {
                    self.overlay_draws.push(draw);
                }
                Ok(())
            }

            0x0a => {
                let p = p
                    .get(..4)
                    .ok_or_else(|| fail("truncated setif parameters"))?;
                let comparison = super::menu::set_if(
                    &mut self.switches,
                    p[0] as usize,
                    p[1] as i32,
                    p[2],
                    p[3] as usize,
                )
                .map_err(fail)?;
                if let Some(variable) = comparison.variable {
                    let saved = self.vm.variables[variable];
                    self.vm.variables[variable] = [u8::from(comparison.value) as f32; 4];
                    let result = self.exec_inputs(doc, root, parent);
                    self.vm.variables[variable] = saved;
                    result
                } else if comparison.execute {
                    self.exec_inputs(doc, root, parent)
                } else {
                    Ok(())
                }
            }
            0x0b => {
                let p = p
                    .get(..5)
                    .ok_or_else(|| fail("truncated state parameters"))?;
                super::menu::state(
                    &mut self.switches,
                    &mut self.game.last_key,
                    p[0] as usize,
                    p[1] as i32,
                    p[2] as i32,
                    p[3] as usize,
                    p[4] as i32,
                )
                .map_err(fail)?;
                self.exec_inputs(doc, root, parent)
            }
            0x0e => {
                let p = p
                    .get(..3)
                    .ok_or_else(|| fail("truncated menu parameters"))?;
                super::menu::menu(
                    &mut self.switches,
                    &mut self.game.last_key,
                    p[0] as usize,
                    p[1] as i32,
                    p[2],
                )
                .map_err(fail)?;
                self.exec_inputs(doc, root, parent)
            }
            0x15 => {
                let values = p
                    .get(..3)
                    .ok_or_else(|| fail("truncated monster override"))?;
                self.next_monster_override = Some(crate::game::monster_actor::Override {
                    kill_switch: values[0] as i32,
                    spawn_switch: values[1] as i32,
                    flags: values[2] as i32,
                });
                let result = self.exec_inputs(doc, root, parent);
                self.next_monster_override = None;
                result
            }
            0x11 => {
                let context = self
                    .actors
                    .clone()
                    .ok_or_else(|| fail("monster scene execution requires collision world"))?;
                let slot = self.instances.monster(root)?;
                let instance = if let Some(instance) = *slot {
                    instance
                } else {
                    let actor = self.game.monsters.allocate().map_err(fail)?;
                    let actor = actor.borrow();
                    let live = self.events.insert_shared(Rc::clone(&actor.event), false);
                    let death = self.events.insert_shared(Rc::clone(&actor.death), false);
                    let instance = super::instance::Monster {
                        actor: actor.id,
                        live,
                        death,
                    };
                    *slot = Some(instance);
                    instance
                };
                let actor = self
                    .game
                    .monsters
                    .get(instance.actor)
                    .ok_or_else(|| fail("monster instance identity missing"))?;
                if actor.borrow().health.state == 0 {
                    let spline = op
                        .splines
                        .first()
                        .copied()
                        .flatten()
                        .map(|id| {
                            doc.splines
                                .get(id)
                                .map(|s| (id, s))
                                .ok_or_else(|| fail("monster spline out of range"))
                        })
                        .transpose()?;
                    let initialized = actor.borrow_mut().initialize(
                        &p,
                        crate::game::monster_actor::Context {
                            world: &mut context.world.borrow_mut(),
                            dynamic: &self.dynamic_cells,
                            player_position: context.player_position,
                            parent,
                            spline,
                            time: self.current_time,
                            time_reset: context.time_reset,
                            flags: self.next_monster_override,
                        },
                    );
                    if let Err(reason) = initialized {
                        actor.borrow_mut().health.state = 0;
                        return Err(fail(reason));
                    }
                    self.instances.forget(Scope::Event(instance.live));
                    self.instances.forget(Scope::Event(instance.death));
                    self.next_monster_override = None;
                }
                let input = *op
                    .inputs
                    .first()
                    .ok_or_else(|| fail("monster has no living input"))?;
                let (alive, state, hit, event_matrix, legs, leg_time) = {
                    let actor = actor.borrow();
                    (
                        actor.health.life > 0,
                        actor.health.state,
                        Rc::clone(&actor.hit_cell),
                        actor.event.borrow().matrix,
                        actor.walk.legs,
                        actor.walk.leg_time,
                    )
                };
                if alive {
                    hit.borrow_mut().matrix1 = event_matrix;
                    self.game.monsters.register(instance.actor).map_err(fail)?;
                    if state >= 3 {
                        if self.dynamic_cells.len() >= 512 {
                            return Err(fail("dynamic cell limit exceeded"));
                        }
                        self.dynamic_cells.push(hit);
                        self.vm.variables[16..24].copy_from_slice(&legs);
                        self.vm.variables[27][0] = leg_time;
                        self.execute_event(doc, input, instance.live, identity())?;
                    }
                } else if let Some(&input) = op.inputs.get(1) {
                    actor.borrow().death.borrow_mut().matrix = event_matrix;
                    self.execute_event(doc, input, instance.death, identity())?;
                }
                Ok(())
            }
            0x07 => {
                if let Some(target) = op.links.first().copied().flatten() {
                    let target_op = doc
                        .operators
                        .get(target)
                        .ok_or_else(|| fail("trigger target out of range"))?;
                    if doc.classes[target_op.class].id == 0x06 {
                        let parameters = super::trigger::Parameters {
                            operator: target,
                            trigger: f(0)?,
                            threshold: f(1)?,
                            delay: f(2)?,
                            duration: f(3)?,
                            flags: *p.get(4).ok_or_else(|| fail("missing trigger flags"))?,
                            velocity: f(5)?,
                            modulation: f(6)?,
                            select: *p.get(7).ok_or_else(|| fail("missing trigger selection"))?
                                as i32,
                            scale: [f(8)?, f(9)?, f(10)?],
                            rotate: [f(11)?, f(12)?, f(13)?],
                            translate: [f(14)?, f(15)?, f(16)?],
                        };
                        if let Some(event) = self.instances.trigger(root)?.advance(
                            self.events.beat,
                            parameters,
                            parent,
                        ) {
                            self.events.insert(event, true);
                        }
                    }
                }
                if let Some(&input) = op.inputs.first() {
                    self.visit(doc, input, parent)?;
                }
                Ok(())
            }
            0x05 | 0x06 => {
                let input = *op
                    .inputs
                    .first()
                    .ok_or_else(|| fail("event operator has no input"))?;
                let mut ids = self.events.active(root).to_vec();
                if id == 0x05 {
                    ids.truncate(1);
                }
                for event_id in ids {
                    let event = self
                        .events
                        .events
                        .get_mut(&event_id)
                        .ok_or_else(|| fail("active event missing"))?;
                    let mut event = event.borrow_mut();
                    if id == 0x06 && event.start == event.end {
                        event.start = self.events.beat;
                        event.end = event
                            .start
                            .wrapping_add((f(0)? * 65536.).round_ties_even() as i32);
                    }
                    drop(event);
                    self.execute_event(doc, input, event_id, parent)?;
                }
                Ok(())
            }

            0x13 => self
                .game
                .register_checkpoint(
                    *p.get(3).ok_or_else(|| fail("missing checkpoint index"))? as usize,
                    [f(0)?, f(1)?, f(2)?],
                    f(4)?,
                )
                .map_err(fail),
            0x14 => {
                let hits = *p.first().ok_or_else(|| fail("missing splash hit amount"))? as i32;
                let range = f(1)?;
                let enable = f(2)?;
                self.splash
                    .trigger(self.instances.splash(root)?, hits, range, enable, parent[3]);
                Ok(())
            }
            0x10 => {
                self.game.apply_parameters(&p).map_err(fail)?;
                self.exec_inputs(doc, root, parent)
            }
            0x12 => {
                let kind = *p.first().ok_or_else(|| fail("missing weapon event kind"))?;
                self.game.events.apply(kind, &op.links).map_err(fail)?;
                self.exec_inputs(doc, root, parent)
            }
            0x08 => {
                let flags = *p.first().ok_or_else(|| fail("missing delay flags"))?;
                let switch = *p.get(1).ok_or_else(|| fail("missing delay switch"))? as usize;
                let switch = if flags & 0xc0 == 0 {
                    *self
                        .switches
                        .get(switch)
                        .ok_or_else(|| fail("delay switch out of range"))?
                } else {
                    0
                };
                let parameters = delay::Parameters {
                    flags,
                    threshold: f(2)?,
                    hysteresis: f(3)?,
                    durations: [f(4)?, f(5)?, f(6)?],
                };
                self.instances.delay(root)?.advance(
                    parameters,
                    switch,
                    &mut self.vm.variables,
                    self.current_time,
                    self.last_time,
                );
                if let Some(&input) = op.inputs.first() {
                    self.visit(doc, input, parent)?;
                }
                Ok(())
            }
            0xce => {
                let flags = *p.first().ok_or_else(|| fail("missing physics flags"))?;
                let scale = [f(4)?, f(5)?, f(6)?];
                let state = self.instances.physic(root, flags, scale, parent)?;
                if let Some(cell) = &state.cell {
                    if flags & 0x10 != 0 {
                        cell.borrow_mut().monster = self.event_monster;
                    }
                    cell.borrow_mut().matrix1 = parent;
                    if self.dynamic_cells.len() >= 512 {
                        return Err(fail("dynamic cell capacity exceeded"));
                    }
                    self.dynamic_cells.push(Rc::clone(cell));
                }
                self.exec_inputs(doc, root, parent)
            }
            0x09 => {
                let switch = *p.first().ok_or_else(|| fail("missing If switch"))? as usize;
                let value = *p.get(1).ok_or_else(|| fail("missing If value"))? as i32;
                let switch = *self
                    .switches
                    .get(switch)
                    .ok_or_else(|| fail("If switch out of range"))?;
                let index = usize::from(switch as i32 != value);
                if let Some(&input) = op.inputs.get(index) {
                    self.visit_scene(doc, input, parent)?;
                }
                Ok(())
            }
            0x02 | 0x03 | 0xc1 => self.inputs(doc, root, parent),
            0xc0 | 0xc3 => self.inputs(doc, root, multiply(srt()?, parent)),
            0xc2 => {
                let count = *p.get(9).ok_or_else(|| fail("missing instance count"))? as i32;
                let step = srt()?;
                let saved = self.vm.variables[3];
                let mut matrix = parent;
                let result = (|| {
                    for i in 0..count {
                        self.vm.variables[3] = [i as f32; 4];
                        self.inputs(doc, root, matrix)?;
                        matrix = multiply(step, matrix);
                    }
                    Ok(())
                })();
                self.vm.variables[3] = saved;
                result
            }
            0xc4 => {
                let local = srt_matrix(&[1., 1., 1., f(0)?, f(1)?, f(2)?, f(3)?, f(4)?, f(5)?]);
                self.jobs.push(Job::Light {
                    operator: root,
                    position: multiply(local, parent)[3],
                    flags: *p.get(6).ok_or_else(|| fail("missing light flags"))?,
                    color: *p.get(8).ok_or_else(|| fail("missing light color"))?,
                    amplify: f(9)?,
                    range: f(10)?,
                    clip: self.current_clip,
                });
                Ok(())
            }
            0xc6 => {
                self.game_camera.space = multiply(srt()?, parent);
                Ok(())
            }
            0xcb => {
                if let Some(&input) = op.inputs.first() {
                    self.jobs.push(Job::Sector {
                        operator: root,
                        input,
                        matrix: parent,
                    });
                }
                Ok(())
            }
            0xcd => {
                let mut links = [None; 3];
                for (n, link) in links.iter_mut().enumerate() {
                    if let Some(i) = op.links.get(n).copied().flatten() {
                        *link = super::cache_scene_identity(doc, i)?;
                    }
                }
                if let [Some(a), Some(b), between] = links {
                    let bounds = [[f(0)?, f(2)?, f(4)?], [f(1)?, f(3)?, f(5)?]];
                    let cost = if f(7)? >= 0.02 {
                        *p.get(6).ok_or_else(|| fail("missing portal cost"))? as i32
                    } else {
                        256
                    };
                    self.jobs.push(Job::Portal {
                        operator: root,
                        portal: Portal::new([a, b], between, bounds, parent, cost),
                    });
                }
                if let Some(&input) = op.inputs.first() {
                    self.visit(doc, input, parent)?;
                }
                Ok(())
            }
            _ => Err(fail("scene execution handler has not been ported")),
        })();
        self.vm
            .restore(eval.saved_variables)
            .map_err(|_| fail("scene variable restore failed"))?;
        result
    }
}

/// Original affine MulA, preserving source evaluation order.
pub fn multiply(a: Matrix, b: Matrix) -> Matrix {
    let mut out = [[0.; 4]; 4];
    for n in 0..4 {
        for axis in 0..3 {
            out[n][axis] = b[0][axis] * a[n][0] + b[1][axis] * a[n][1] + b[2][axis] * a[n][2];
        }
    }
    for axis in 0..3 {
        out[3][axis] += b[3][axis];
    }
    out[3][3] = 1.;
    out
}
