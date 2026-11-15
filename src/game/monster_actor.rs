//! Original monster scene initialization, including the parameter-copy spill.
use super::{monster::Health, monster_walk};
use crate::{
    collision::{Cell, DynamicCell, world::World},
    kx::Spline,
    mesh::{Matrix, identity},
    scene::event::Event,
};
use std::{cell::RefCell, rc::Rc};
#[derive(Clone, Debug, Default)]
pub struct Parameters {
    pub speed: f32,
    pub notice_radius: f32,
    pub charge_radius: f32,
    pub flags: i32,
    pub life_max: i32,
    pub melee_hits: i32,
    pub melee_time: f32,
    pub ranged_hits: i32,
    pub ranged_time: f32,
    pub ranged_aim: f32,
    pub wait_radius: f32,
    pub min_radius: f32,
}
#[derive(Clone, Debug)]
pub struct Body {
    pub position: [f32; 4],
    pub old_position: [f32; 4],
    pub cell: Option<usize>,
    pub radius: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct Override {
    pub kill_switch: i32,
    pub spawn_switch: i32,
    pub flags: i32,
}
pub struct Context<'a> {
    pub world: &'a mut World,
    pub dynamic: &'a [Rc<RefCell<DynamicCell>>],
    pub player_position: [f32; 4],
    pub parent: Matrix,
    pub spline: Option<(usize, &'a Spline)>,
    pub time: i32,
    pub time_reset: bool,
    pub flags: Option<Override>,
}
pub struct Actor {
    pub id: usize,
    pub health: Health,
    pub parameters: Parameters,
    pub matrix: Matrix,
    pub event: crate::scene::event::Handle,
    pub death: crate::scene::event::Handle,
    pub melee_timer: f32,
    pub ranged_timer: f32,
    pub walk_style: usize,
    pub weapon_kind: i32,
    pub spline: Option<usize>,
    pub magnet_force: [f32; 4],
    pub position_offset: [f32; 4],
    pub body: Body,
    pub hit_cell: Rc<RefCell<DynamicCell>>,
    pub walk: monster_walk::State,
    pub spawn_switch: i32,
}
impl Actor {
    pub fn uninitialized(id: usize) -> Self {
        Self {
            id,
            health: Health {
                life: 0,
                armor: 0,
                state: 0,
                kill_switch: 0,
            },
            parameters: Parameters::default(),
            matrix: [[0.; 4]; 4],
            event: Rc::new(RefCell::new(Event::default())),
            death: Rc::new(RefCell::new(Event::default())),
            melee_timer: 0.,
            ranged_timer: 0.,
            walk_style: 0,
            weapon_kind: 0,
            spline: None,
            magnet_force: [0.; 4],
            position_offset: [0.; 4],
            body: Body {
                position: [0.; 4],
                old_position: [0.; 4],
                cell: None,
                radius: 0.,
            },
            hit_cell: Rc::new(RefCell::new(DynamicCell::new(
                Cell::new(identity(), [1.; 3], 2),
                identity(),
            ))),
            walk: monster_walk::State::default(),
            spawn_switch: 0,
        }
    }
    pub fn spawn(id: usize, words: &[u32], context: Context<'_>) -> Result<Self, &'static str> {
        let mut actor = Self::uninitialized(id);
        actor.initialize(words, context)?;
        Ok(actor)
    }
    /// Reinitialization preserves attack timers, magnet force and the stable hit
    /// cell, as the original KMS_RESPAWN branch does. Event values are reset.
    pub fn initialize(&mut self, words: &[u32], context: Context<'_>) -> Result<(), &'static str> {
        let p = words.get(..29).ok_or("truncated monster parameters")?;
        let f = |i| f32::from_bits(p[i]);
        let int = |i| p[i] as i32;
        let style = p[26] as usize;
        let definition = monster_walk::DEFINITIONS
            .get(style)
            .ok_or("monster walk style out of range")?;
        *self.event.borrow_mut() = Event::default();
        self.parameters = Parameters {
            speed: f(6) / 1000.,
            notice_radius: f(7),
            charge_radius: f(8),
            flags: int(9),
            life_max: int(10),
            melee_hits: int(12),
            melee_time: f(13),
            ranged_hits: int(14),
            ranged_time: f(15),
            ranged_aim: f(16),
            wait_radius: f(18),
            min_radius: f(19),
        };
        self.health.armor = int(11);
        self.health.kill_switch = int(17);
        // The source copies 16 words into a 14-word parameter block. The final two
        // words overwrite Event.Matrix.i.x/y after Event.Init; preserve that layout.
        self.event.borrow_mut().matrix[0][0] = f(20);
        self.event.borrow_mut().matrix[0][1] = f(21);
        self.walk_style = style;
        self.weapon_kind = int(27);
        self.spline = context.spline.map(|(id, _)| id);
        self.health.life = self.parameters.life_max;
        self.health.state = 1;
        self.spawn_switch = int(28);
        self.event.borrow_mut().monster = Some(self.id);
        *self.death.borrow_mut() = Event::default();
        self.position_offset = [f(23), f(24), f(25), 0.];
        self.matrix = context.parent;
        let mut delta = [
            self.matrix[3][0] - context.player_position[0],
            0.,
            self.matrix[3][2] - context.player_position[2],
            0.,
        ];
        let scale = (1.
            / f64::from(delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt())
            as f32;
        for x in &mut delta[..3] {
            *x *= scale;
        }
        self.matrix[0] = [-delta[2], 0., delta[0], 0.];
        self.matrix[1] = [0., 1., 0., 0.];
        self.matrix[2] = [-delta[0], 0., -delta[2], 0.];
        self.body.position = self.matrix[3];
        self.body.cell = context.world.find_cell(self.body.position);
        self.body.radius = 0.25;
        self.body.old_position = self.body.position;
        if self.body.cell.is_none() {
            self.health.life = 0;
        }
        let mut local = identity();
        for axis in 0..3 {
            local[3][axis] += self.position_offset[axis];
        }
        let cell = Cell::new(local, [f(20), f(21), f(22)], 2);
        let mut hit = DynamicCell::new(cell, self.matrix);
        hit.monster = Some(self.id);
        *self.hit_cell.borrow_mut() = hit;
        self.walk.reset = true;
        self.matrix[3] = self.body.position;
        self.walk.advance(
            definition,
            &mut self.matrix,
            monster_walk::Context {
                world: context.world,
                dynamic: context.dynamic,
                spline: context.spline.map(|(_, s)| s),
                cell: self.body.cell,
                time: context.time,
                time_reset: context.time_reset,
            },
        )?;
        if let Some(flags) = context.flags {
            self.health.kill_switch = flags.kill_switch;
            self.spawn_switch = flags.spawn_switch;
            self.parameters.flags |= flags.flags;
        }
        Ok(())
    }
}
