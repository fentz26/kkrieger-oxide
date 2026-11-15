//! Original KKMonsterWalk foot placement, gait timing and body fitting.
use crate::{
    collision::{DynamicCell, ray::Info, world::World},
    kx::Spline,
    mesh::Matrix,
};
use std::{cell::RefCell, rc::Rc};
type V = [f32; 4];
#[derive(Clone, Copy, Debug)]
pub struct Definition {
    pub flags: u32,
    pub foot_count: usize,
    pub step_threshold: f32,
    pub run_low: f32,
    pub run_high: f32,
    pub scan_up: f32,
    pub scan_down: f32,
    pub group: [usize; 2],
    pub length: [f32; 2],
    pub speed: [f32; 2],
    pub next: [f32; 2],
    pub legs: [[f32; 3]; 8],
}
#[derive(Clone, Debug)]
pub struct State {
    pub reset: bool,
    pub foot_cells: [Option<usize>; 8],
    pub old: [V; 8],
    pub new: [V; 8],
    pub step_time: [i32; 8],
    pub last_time: i32,
    pub old_position: V,
    pub new_position: V,
    pub center: V,
    pub state: usize,
    pub last_leg: usize,
    pub next_leg: usize,
    pub idle: bool,
    pub deviation: V,
    pub deviation_flag: i32,
    pub init_steps: i32,
    pub delta: [V; 8],
    pub legs: [V; 8],
    pub leg_time: f32,
}
impl Default for State {
    fn default() -> Self {
        Self {
            reset: true,
            foot_cells: [None; 8],
            old: [[0.; 4]; 8],
            new: [[0.; 4]; 8],
            step_time: [0; 8],
            last_time: 0,
            old_position: [0.; 4],
            new_position: [0.; 4],
            center: [0.; 4],
            state: 0,
            last_leg: 0,
            next_leg: 0,
            idle: false,
            deviation: [0.; 4],
            deviation_flag: 0,
            init_steps: 0,
            delta: [[0.; 4]; 8],
            legs: [[0.; 4]; 8],
            leg_time: 0.,
        }
    }
}
pub struct Context<'a> {
    pub world: &'a mut World,
    pub dynamic: &'a [Rc<RefCell<DynamicCell>>],
    pub spline: Option<&'a Spline>,
    pub cell: Option<usize>,
    pub time: i32,
    pub time_reset: bool,
}
fn sub(a: V, b: V) -> V {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2], 0.]
}
fn add_scaled(a: &mut V, b: V, s: f32) {
    for i in 0..3 {
        a[i] += b[i] * s;
    }
}
fn length(v: V) -> f32 {
    f64::from(v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt() as f32
}
fn unit(v: &mut V) {
    let f = (1. / f64::from(v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()) as f32;
    for x in &mut v[..3] {
        *x *= f;
    }
}
fn cross(a: V, b: V) -> V {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
        0.,
    ]
}
fn rotate(m: Matrix, v: V) -> V {
    let mut r = [0.; 4];
    for i in 0..4 {
        r[i] = m[0][i] * v[0];
        r[i] += m[1][i] * v[1];
        r[i] += m[2][i] * v[2];
    }
    r
}
fn rotate34(m: Matrix, v: V) -> V {
    let mut r = rotate(m, v);
    for i in 0..4 {
        r[i] += m[3][i];
    }
    r
}
impl State {
    pub fn advance(
        &mut self,
        wi: &Definition,
        matrix: &mut Matrix,
        c: Context<'_>,
    ) -> Result<(), &'static str> {
        if wi.foot_count == 0
            || wi.foot_count > 8
            || wi.group.into_iter().any(|g| g == 0 || g > wi.foot_count)
            || self.state > 1
        {
            return Err("invalid monster walk definition");
        }
        let speed = wi.speed.map(|f| (f * 1000.).round_ties_even() as i32);
        let next = std::array::from_fn::<_, 2, _>(|i| {
            ((wi.next[i] * 1000.).round_ties_even() as i32).min(speed[i])
        });
        if self.reset || c.time_reset {
            self.reset = false;
            self.center = [0., 0., 0., 1.];
            self.init_steps = wi.foot_count as i32 * 2;
            for i in 0..wi.foot_count {
                self.delta[i] = [wi.legs[i][0], wi.legs[i][1], wi.legs[i][2], 1.];
                let mut p0 = rotate34(*matrix, self.delta[i]);
                p0[3] = 1.;
                let mut p1 = p0;
                p1[1] -= 2.;
                let mut v = p0;
                let mut cell = c.cell;
                let mut info = Info::default();
                if let Some(id) = cell.as_mut()
                    && c.world.collide_ray(id, p1, p0, &mut info, c.dynamic)?
                {
                    v = info.position;
                }
                v[3] = 0.;
                self.old[i] = v;
                self.new[i] = v;
                self.step_time[i] = speed[0];
                self.foot_cells[i] = cell;
                self.center[0] += self.delta[i][0] / wi.foot_count as f32;
                self.center[2] += self.delta[i][2] / wi.foot_count as f32;
            }
            self.last_time = c.time;
            self.new_position = matrix[3];
            self.state = 0;
            self.last_leg = 0;
            for delta in &mut self.delta[..wi.foot_count] {
                for i in 0..3 {
                    delta[i] -= self.center[i];
                }
            }
        }
        let mut state = self.state;
        let mut last_leg = self.last_leg;
        let mut foot = [[0.; 4]; 8];
        let mut spline = [[0.; 4]; 8];
        let mut sum = [0., 0., 0., 1.];
        let mut f = 0.;
        for i in 0..wi.foot_count {
            let t = (self.step_time[i] as f32 / speed[state] as f32).clamp(0., 1.);
            if i < wi.group[state] {
                f += t;
            }
            if let Some(curve) = c.spline {
                spline[i] = curve.evaluate((t + state as f32) * 0.5);
                if self.delta[i][0] < 0. {
                    spline[i][0] = -spline[i][0];
                }
                if self.delta[i][2] < 0. {
                    spline[i][2] = -spline[i][2];
                }
                spline[i][3] -= state as f32;
            } else {
                spline[i] = [
                    0.,
                    ((f64::from(t) * std::f64::consts::PI).sin() * 0.25) as f32,
                    0.,
                    t,
                ];
            }
            for axis in 0..3 {
                foot[i][axis] =
                    self.old[i][axis] + (self.new[i][axis] - self.old[i][axis]) * spline[i][3];
            }
            foot[i][3] = 1.;
            for axis in 0..3 {
                sum[axis] += foot[i][axis];
            }
        }
        for value in &mut sum[..3] {
            *value *= 1. / wi.foot_count as f32;
        }
        let mut mat = [[0.; 4]; 4];
        mat[3] = [sum[0], sum[1], sum[2], 1.];
        for (i, &position) in foot[..wi.foot_count].iter().enumerate() {
            let v = sub(position, sum);
            for (axis, basis) in mat[..3].iter_mut().enumerate() {
                add_scaled(basis, v, self.delta[i][axis]);
            }
        }
        match (wi.flags >> 2) & 3 {
            0 => mat[..3].copy_from_slice(&matrix[..3]),
            2 => {
                unit(&mut mat[0]);
                mat[1] = [0., 1., 0., 0.];
                mat[2] = cross(mat[0], mat[1]);
                unit(&mut mat[2]);
                mat[1] = cross(mat[2], mat[0]);
                unit(&mut mat[1]);
            }
            mode => {
                if mode == 1 {
                    mat[2] = matrix[2];
                }
                unit(&mut mat[2]);
                mat[1] = cross(mat[2], mat[0]);
                unit(&mut mat[1]);
                mat[0] = cross(mat[1], mat[2]);
                unit(&mut mat[0]);
            }
        }
        let mut t = (f + last_leg as f32) / wi.group[state] as f32;
        if t >= 2. {
            t -= 2.;
        }
        if t >= 1. {
            t -= 1.;
        }
        self.leg_time = (t + state as f32) * 0.5;
        let time = c.time.wrapping_sub(self.last_time).max(0);
        self.last_time = c.time;
        let mut dir = *matrix;
        if self.deviation_flag > 0 {
            for axis in 0..3 {
                dir[3][axis] = mat[3][axis] + self.deviation[axis];
            }
        }
        dir[2][1] = 0.;
        unit(&mut dir[2]);
        dir[1] = [0., 1., 0., 0.];
        dir[0] = cross(dir[1], dir[2]);
        for timer in &mut self.step_time[..wi.foot_count] {
            if *timer < speed[state] {
                *timer = timer.wrapping_add(time);
            }
        }
        let mut dist = 0.;
        let mut next_leg = (last_leg + 1) % wi.group[state];
        let mut best_leg = next_leg;
        for i in 0..wi.foot_count {
            let mut v = sub(rotate34(dir, self.delta[i]), foot[i]);
            v[1] = 0.;
            let f = length(v);
            if f > dist && self.step_time[i] >= speed[state] {
                dist = f;
                best_leg = i;
            }
            let v = rotate(dir, spline[i]);
            for axis in 0..3 {
                self.legs[i][axis] = foot[i][axis] + v[axis];
            }
            self.legs[i][3] = spline[i][3];
        }
        let mut change = 1;
        for timer in &self.step_time[..wi.foot_count] {
            if *timer < speed[state] {
                change = 0;
            }
        }
        let changing = (state == 0 && dist > wi.run_high) || (state == 1 && dist < wi.run_low);
        if changing {
            change += 2;
        }
        if change == 3 && changing {
            state = 1 - state;
            change = 0;
            last_leg %= wi.group[state];
            self.step_time[..wi.foot_count].fill(speed[state]);
            next_leg = best_leg % wi.group[state];
        }
        best_leg %= wi.group[state];
        if self.idle && change == 1 {
            next_leg = best_leg;
        }
        self.idle = false;
        if change != 2
            && self.step_time[last_leg] >= next[state]
            && self.step_time[next_leg] >= speed[state]
        {
            if self.init_steps > 0 {
                self.init_steps -= 1;
                dist = 1024.;
            }
            if self.deviation_flag > 0 {
                self.deviation_flag -= 1;
            }
            if dist > wi.step_threshold {
                let v = sub(dir[3], mat[3]);
                let f = length(v);
                if wi.flags & 1 != 0 && self.init_steps == 0 {
                    next_leg = best_leg;
                }
                if f > wi.length[state] {
                    self.new_position = mat[3];
                    add_scaled(&mut self.new_position, v, wi.length[state] / f);
                } else {
                    self.new_position = dir[3];
                }
                dir[3] = self.new_position;
                let j = self.step_time[last_leg].wrapping_sub(next[state]);
                for i in (next_leg..wi.foot_count).step_by(wi.group[state]) {
                    self.old[i] = foot[i];
                    let mut v = self.delta[i];
                    v[1] = 0.;
                    v = rotate34(dir, v);
                    self.new[i] = v;
                    self.step_time[i] = j;
                    if c.cell.is_some() {
                        if self.foot_cells[i].is_none() {
                            self.foot_cells[i] = c.cell;
                        }
                        let mut p0 = v;
                        p0[1] += wi.scan_up;
                        let mut p1 = v;
                        p1[1] -= wi.scan_down;
                        let mut info = Info::default();
                        if let Some(cell) = self.foot_cells[i].as_mut()
                            && c.world.collide_ray(cell, p1, p0, &mut info, c.dynamic)?
                            && info.plane[1] > 0.75
                            && info.position[1] < p0[1]
                        {
                            self.new[i] = info.position;
                            self.step_time[i] = j;
                        }
                    }
                }
                last_leg = next_leg;
            } else {
                self.idle = true;
            }
        }
        *matrix = mat;
        self.state = state;
        self.last_leg = last_leg;
        self.next_leg = next_leg;
        Ok(())
    }
}

pub const DEFINITIONS: [Definition; 5] = [
    Definition {
        flags: 2,
        foot_count: 3,
        step_threshold: 0.15,
        run_low: 0.75,
        run_high: 1.25,
        scan_up: 2.0,
        scan_down: 2.0,
        group: [3, 3],
        length: [0.87, 1.25],
        speed: [0.39, 0.29],
        next: [0.5, 0.35],
        legs: [
            [-0.5, 0.125, 0.0],
            [0.5, 0.125, 0.0],
            [0.0, 0.205, 0.75],
            [0., 0., 0.],
            [0., 0., 0.],
            [0., 0., 0.],
            [0., 0., 0.],
            [0., 0., 0.],
        ],
    },
    Definition {
        flags: 2,
        foot_count: 6,
        step_threshold: 0.15,
        run_low: 0.75,
        run_high: 1.25,
        scan_up: 2.0,
        scan_down: 2.0,
        group: [6, 6],
        length: [1.255, 2.16],
        speed: [0.32, 0.155],
        next: [0.545, 0.21],
        legs: [
            [-1.125, 0.0, 0.0],
            [1.125, 0.0, 0.0],
            [-0.625, 0.0, 0.0],
            [0.625, 0.0, 0.0],
            [-1.0, 0.0, -0.125],
            [1.0, 0.0, -0.125],
            [0., 0., 0.],
            [0., 0., 0.],
        ],
    },
    Definition {
        flags: 2,
        foot_count: 4,
        step_threshold: 0.15,
        run_low: 0.75,
        run_high: 1.25,
        scan_up: 2.0,
        scan_down: 2.0,
        group: [4, 4],
        length: [1.13, 2.16],
        speed: [0.27, 0.17],
        next: [0.51, 0.26],
        legs: [
            [-0.6, 0.25, 0.5],
            [0.6, 0.25, 0.5],
            [-0.45, 0.03, -0.86],
            [0.45, 0.03, -0.875],
            [0., 0., 0.],
            [0., 0., 0.],
            [0., 0., 0.],
            [0., 0., 0.],
        ],
    },
    Definition {
        flags: 2,
        foot_count: 2,
        step_threshold: 0.15,
        run_low: 1.75,
        run_high: 1.25,
        scan_up: 2.0,
        scan_down: 2.0,
        group: [2, 2],
        length: [0.505, 1.47],
        speed: [0.92, 0.8],
        next: [1.125, 0.96],
        legs: [
            [-0.8, 0.125, 0.0],
            [0.8, 0.125, 0.0],
            [0., 0., 0.],
            [0., 0., 0.],
            [0., 0., 0.],
            [0., 0., 0.],
            [0., 0., 0.],
            [0., 0., 0.],
        ],
    },
    Definition {
        flags: 2,
        foot_count: 2,
        step_threshold: 0.15,
        run_low: 1.75,
        run_high: 1.25,
        scan_up: 2.0,
        scan_down: 2.0,
        group: [2, 2],
        length: [0.505, 1.47],
        speed: [0.92, 0.8],
        next: [1.125, 0.96],
        legs: [
            [-0.8, 0.125, 0.0],
            [0.8, 0.125, 0.0],
            [0., 0., 0.],
            [0., 0., 0.],
            [0., 0., 0.],
            [0., 0., 0.],
            [0., 0., 0.],
            [0., 0., 0.],
        ],
    },
];
