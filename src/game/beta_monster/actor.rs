//! Released actor spawn composition. Scene execution and AI registration remain
//! distinct from constructing persistent actor/body/hit-cell storage.
use super::{
    HitBox, InitialState,
    body::Body,
    dynamic::{MovingCell, Pose},
};
use crate::mesh::Matrix;
#[derive(Clone, Debug)]
pub struct Actor {
    pub initial: InitialState,
    pub body: Body,
    pub hit_cell: MovingCell,
    pub active: bool,
    /// Melee and ranged attack progress, respectively.
    pub cooldowns: [f32; 2],
}
impl Actor {
    pub fn spawn<F>(
        words: &[u32],
        parent: Matrix,
        spline_token: u32,
        actor_token: u32,
        mut find_cell: F,
    ) -> Result<Self, &'static str>
    where
        F: FnMut([f32; 4]) -> Option<usize>,
    {
        let initial = InitialState::decode(words, parent, spline_token, actor_token)?;
        let hull = HitBox::decode(words)?.collision_hull();
        if hull.count > 6 {
            return Err("dynamic plane capacity exceeded");
        }
        let mut original = [[0.; 4]; 6];
        original[..hull.count].copy_from_slice(&hull.planes[..hull.count]);
        let cell = find_cell(initial.cell_query_position);
        let body = Body::new(
            initial.cell_query_position,
            cell,
            initial.collider_type as i32,
        );
        // Released PrepareDynamic copies planes and matrices but does not apply
        // the matrix yet. First PhysicDCell acceptance publishes world geometry.
        let hit_cell = MovingCell {
            original,
            count: hull.count,
            confirmed: parent,
            current: parent,
            mode: hull.mode | 0x20,
            monster: true,
            pose: Pose {
                planes: original,
                count: hull.count,
                center: hull.center,
                matrix: parent,
                blocked: false,
            },
        };
        Ok(Self {
            initial,
            body,
            hit_cell,
            active: false,
            cooldowns: [0.; 2],
        })
    }
}

impl Actor {
    /// Advance the active branch's attack progress. The caller applies the
    /// AI's life check before entering the per-tick branches.
    pub fn advance_cooldowns(&mut self) {
        if !self.active {
            return;
        }
        for (timer, parameter) in self.cooldowns.iter_mut().zip([7, 9]) {
            let period = f64::from(f32::from_bits(self.initial.copied.parameters[parameter]));
            let numerator = f64::from(f32::from_bits(0x3c23d70a));
            let q = numerator / period;
            let increment = super::rounded_arithmetic(q, (-q).mul_add(period, numerator) / period);
            let next = super::add(increment, f64::from(*timer));
            *timer = if next >= 1. { 1. } else { super::down(next) };
        }
    }

    /// Released AI notice-radius gate. Visibility is queried only across a
    /// strict radius boundary; failed activation leaves particle cells intact.
    pub fn update_visibility<V, F>(&mut self, distance: f32, mut blocked: V, mut find_cell: F)
    where
        V: FnMut() -> bool,
        F: FnMut([f32; 4]) -> Option<usize>,
    {
        let radius = f32::from_bits(self.initial.copied.parameters[1]);
        if !self.active {
            if distance < radius && !blocked() {
                self.activate(&mut find_cell);
            }
        } else if distance > radius && blocked() {
            self.active = false;
        }
    }

    fn activate<F>(&mut self, find_cell: &mut F)
    where
        F: FnMut([f32; 4]) -> Option<usize>,
    {
        if !self.active
            && let Some(cell) = find_cell(self.body.particles[0].vectors[0])
        {
            self.active = true;
            for particle in &mut self.body.particles {
                particle.cell = Some(cell);
            }
        }
    }

    /// Released monster Hit: activate/reacquire first, then armor damage and
    /// wrapping kill-switch update. Missing world cells do not suppress damage.
    pub fn hit<F>(
        &mut self,
        hits: i32,
        switches: &mut [u8; 256],
        mut find_cell: F,
    ) -> Result<(), &'static str>
    where
        F: FnMut([f32; 4]) -> Option<usize>,
    {
        if self.initial.life <= 0 {
            return Ok(());
        }
        let kill = self.initial.copied.parameters[11] as i32;
        if kill > 255 {
            return Err("monster kill switch out of range");
        }
        self.activate(&mut find_cell);
        let armor = self.initial.copied.parameters[5] as i32;
        let delta = if hits > armor {
            armor.wrapping_sub(armor / 4).wrapping_sub(hits)
        } else {
            hits / -4
        };
        self.initial.life = self.initial.life.wrapping_add(delta);
        if self.initial.life <= 0 {
            self.initial.life = 0;
            if kill > 0 {
                switches[kill as usize] = switches[kill as usize].wrapping_add(1);
            }
        }
        Ok(())
    }
}
