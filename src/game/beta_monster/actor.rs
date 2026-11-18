//! Released actor spawn composition. Scene execution and AI registration remain
//! distinct from constructing persistent actor/body/hit-cell storage.
use super::{
    HitBox, InitialState,
    body::Body,
    dynamic::{MovingCell, Pose},
};
use crate::mesh::Matrix;
/// Attack effects to apply to the player or enqueue in the released event list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Attack {
    Melee(i32),
    Ranged([u32; 45]),
}

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
        attack_operator: u32,
        actor_token: u32,
        mut find_cell: F,
    ) -> Result<Self, &'static str>
    where
        F: FnMut([f32; 4]) -> Option<usize>,
    {
        let initial = InitialState::decode(words, parent, attack_operator, actor_token)?;
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
    /// Build the displacement with the released add-then-subtract spills.
    pub fn steering_displacement(&self, distance: f32, direction: [f32; 4]) -> [f32; 4] {
        use super::{add, down, mul};
        let scale = f64::from(self.steering_scale(distance));
        let position = self.body.particles[0].vectors[0];
        let mut delta = position;
        for axis in 0..3 {
            let target = down(add(
                mul(scale, f64::from(direction[axis])),
                f64::from(position[axis]),
            ));
            delta[axis] = down(add(f64::from(target), -f64::from(position[axis])));
        }
        delta
    }

    /// Horizontal vector from the player to the main particle and its length.
    pub fn player_direction(&self, player: [f32; 4]) -> ([f32; 4], f32) {
        use super::{add, down, mul, rounded_arithmetic};
        let position = self.body.particles[0].vectors[0];
        let mut direction = [0.; 4];
        for axis in [0, 2] {
            direction[axis] = down(add(f64::from(position[axis]), -f64::from(player[axis])));
        }
        let x = f64::from(direction[0]);
        let z = f64::from(direction[2]);
        let squared = add(add(mul(z, z), 0.), mul(x, x));
        let root = squared.sqrt();
        let length = rounded_arithmetic(root, (-root).mul_add(root, squared));
        if length > 1e-6 {
            let quotient = 1. / length;
            let inverse = down(rounded_arithmetic(
                quotient,
                (-quotient).mul_add(length, 1.) / length,
            ));
            for value in &mut direction[..3] {
                *value = down(mul(f64::from(*value), f64::from(inverse)));
            }
        } else {
            direction[..3].copy_from_slice(&[1., 0., 0.]);
        }
        (direction, down(length))
    }

    /// Copy the current physics position and horizontal facing into the event.
    pub fn update_event_pose(&mut self, direction: [f32; 4]) {
        self.initial.event_words[21..25]
            .copy_from_slice(&self.body.particles[0].vectors[0].map(f32::to_bits));
        self.initial.event_words[17..21].fill(0);
        let angle = (-f64::from(direction[0])).atan2(-f64::from(direction[2]));
        self.initial.event_words[18] =
            super::down(super::mul(angle, 0.15915493866300567)).to_bits();
    }

    /// Publish life modulation and the released event selection mask.
    pub fn update_health_modulation(&mut self) {
        let numerator = f64::from(self.initial.life);
        let denominator = f64::from(self.initial.copied.parameters[4] as i32);
        let quotient = numerator / denominator;
        let ratio = if numerator == 0. && denominator == 0. {
            f64::from_bits(0xfff8_0000_0000_0000)
        } else {
            super::rounded_arithmetic(
                quotient,
                (-quotient).mul_add(denominator, numerator) / denominator,
            )
        };
        self.initial.event_words[15] = super::down(ratio).to_bits();
        self.initial.event_words[44] = 0x23;
    }

    /// Write the attack phases consumed by the released event operators.
    pub fn update_attack_modulation(&mut self) {
        for (index, timer) in self.cooldowns.into_iter().enumerate() {
            let mut phase = super::add(f64::from(timer), -0.5);
            if phase < 0. {
                phase = super::add(phase, 1.);
            }
            self.initial.event_words[13 + index] = super::down(phase).to_bits();
        }
    }

    /// Apply the released vertical correction using the downward cast clearance.
    pub fn correct_height(
        vertical: f32,
        clearance: f32,
        gravity: f32,
        height: f32,
        gain: f32,
    ) -> f32 {
        use super::{add, down, mul};
        let gravity = f64::from(gravity);
        let height = f64::from(height);
        if add(gravity, height) > f64::from(clearance) {
            let error = add(add(height, -gravity), -f64::from(clearance));
            down(add(
                add(mul(error, f64::from(gain)), gravity),
                f64::from(vertical),
            ))
        } else {
            vertical
        }
    }

    /// Signed scale applied to the horizontal monster-to-player vector.
    pub fn steering_scale(&self, distance: f32) -> f32 {
        let parameter = |i| f64::from(f32::from_bits(self.initial.copied.parameters[i]));
        let speed = parameter(0);
        let target = if f64::from(distance) < parameter(2) {
            parameter(13)
        } else {
            parameter(12)
        };
        let mut scale = super::add(f64::from(distance), -target);
        if scale >= speed {
            scale = speed;
        } else if scale < -speed {
            scale = -speed;
        }
        super::down(-scale)
    }

    /// Select an attack after advancing cooldowns. The ranged visibility ray
    /// starts at the player and ends at the actor matrix translation, each
    /// raised by one unit. Its progress resets even when the ray is blocked.
    pub fn select_attack<F>(
        &mut self,
        distance: f32,
        player_position: [f32; 4],
        mut blocked: F,
    ) -> Option<Attack>
    where
        F: FnMut([f32; 4], [f32; 4]) -> bool,
    {
        if !self.active || self.cooldowns != [1., 1.] {
            return None;
        }
        if distance < 1.5 {
            let hits = self.initial.copied.parameters[6] as i32;
            if hits <= 0 {
                return None;
            }
            self.cooldowns[0] = 0.;
            Some(Attack::Melee(hits))
        } else {
            if self.initial.copied.parameters[8] as i32 <= 0 || self.initial.attack_operator == 0 {
                return None;
            }
            self.cooldowns[1] = 0.;
            let mut start = player_position;
            let mut end = self.initial.matrix[3];
            start[1] = super::down(super::add(f64::from(start[1]), 1.));
            end[1] = super::down(super::add(f64::from(end[1]), 1.));
            if blocked(start, end) {
                return None;
            }
            let mut event = [0; 45];
            event[0] = self.initial.attack_operator;
            event[17..25].copy_from_slice(&self.initial.event_words[17..25]);
            event[43] = self.initial.event_words[43];
            Some(Attack::Ranged(event))
        }
    }

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
