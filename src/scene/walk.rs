//! Published sINTRO Scene_Walk gait state and child-visible leg variables.
use crate::{kx::Spline, mesh::Matrix};
type Vector = [f32; 4];
pub struct Parameters {
    pub flags: u32,
    pub count: usize,
    pub threshold: f32,
    pub run_low: f32,
    pub run_high: f32,
    pub groups: [usize; 2],
    pub length: [f32; 2],
    pub speed: [i32; 2],
    pub next: [i32; 2],
    pub feet: [[f32; 3]; 8],
}
impl Parameters {
    pub fn from_words(p: &[u32]) -> Result<Self, &'static str> {
        let p = p.get(..39).ok_or("truncated walker parameters")?;
        let f = |i| f32::from_bits(p[i]);
        let speed = std::array::from_fn(|i| (f(9 + i) * 1000.).round_ties_even() as i32);
        Ok(Self {
            flags: p[0],
            count: p[1] as usize,
            threshold: f(2),
            run_low: f(3),
            run_high: f(4),
            groups: [p[5] as usize, p[6] as usize],
            length: [f(7), f(8)],
            speed,
            next: std::array::from_fn(|i| {
                ((f(11 + i) * 1000.).round_ties_even() as i32).min(speed[i])
            }),
            feet: std::array::from_fn(|i| std::array::from_fn(|axis| f(13 + i * 3 + axis))),
        })
    }
}
#[derive(Default)]
pub struct State {
    initialized: bool,
    pub old: [Vector; 8],
    pub new: [Vector; 8],
    pub step_time: [i32; 8],
    pub last_time: i32,
    pub new_position: Vector,
    pub center: Vector,
    pub state: usize,
    pub last_leg: usize,
    pub next_leg: usize,
    pub idle: bool,
    pub deviation: Vector,
    pub deviation_flag: i32,
    pub initial_steps: i32,
    pub delta: [Vector; 8],
}
pub struct Frame {
    pub transform: Matrix,
    pub time: Vector,
    pub feet: [Vector; 8],
    pub count: usize,
}
impl State {
    pub fn advance(
        &mut self,
        p: &Parameters,
        parent: Matrix,
        now: i32,
        reset: bool,
        spline: Option<&Spline>,
    ) -> Result<Frame, &'static str> {
        if p.count == 0
            || p.count > 8
            || p.groups.iter().any(|&g| g == 0 || g > p.count)
            || p.speed.iter().any(|&s| s <= 0)
        {
            return Err("invalid walker foot groups or duration");
        }
        if !self.initialized || p.flags & 0x100 != 0 || reset {
            self.center = [0., 0., 0., 1.];
            self.initial_steps = p.count as i32 * 2;
            for i in 0..p.count {
                self.delta[i] = [p.feet[i][0], p.feet[i][1], p.feet[i][2], 0.];
                let mut v = add(parent[3], self.delta[i]);
                v[3] = 0.;
                self.old[i] = v;
                self.new[i] = v;
                self.step_time[i] = p.speed[0];
                self.center[0] += self.delta[i][0] / p.count as f32;
                self.center[2] += self.delta[i][2] / p.count as f32;
            }
            self.last_time = now;
            self.new_position = parent[3];
            self.state = 0;
            self.last_leg = 0;
            for i in 0..p.count {
                self.delta[i] = sub(self.delta[i], self.center);
            }
            self.initialized = true;
        }
        let mut state = self.state;
        let mut last = self.last_leg;
        let mut foot = [[0.; 4]; 8];
        let mut offsets = [[0.; 4]; 8];
        let mut sum = [0., 0., 0., 1.];
        let mut phase = 0.;
        for i in 0..p.count {
            let t = (self.step_time[i] as f32 / p.speed[state] as f32).clamp(0., 1.);
            if i < p.groups[state] {
                phase += t;
            }
            offsets[i] = if let Some(spline) = spline {
                let mut v = spline.evaluate((t + state as f32) * 0.5);
                if self.delta[i][0] < 0. {
                    v[0] = -v[0];
                }
                if self.delta[i][2] < 0. {
                    v[2] = -v[2];
                }
                v[3] -= state as f32;
                v
            } else {
                [
                    0.,
                    ((f64::from(t) * std::f64::consts::PI).sin() * 0.25) as f32,
                    0.,
                    t,
                ]
            };
            foot[i] = std::array::from_fn(|axis| {
                self.old[i][axis] + (self.new[i][axis] - self.old[i][axis]) * offsets[i][3]
            });
            foot[i][3] = 1.;
            sum = add(sum, foot[i]);
        }
        sum = scale(sum, 1. / p.count as f32);
        let mut mat = [[0.; 4]; 4];
        mat[3] = [sum[0], sum[1], sum[2], 1.];
        for (i, &point) in foot[..p.count].iter().enumerate() {
            let v = sub(point, sum);
            for (axis, basis) in mat[..3].iter_mut().enumerate() {
                *basis = add_scaled(*basis, v, self.delta[i][axis]);
            }
        }
        match (p.flags >> 2) & 3 {
            0 => mat[..3].copy_from_slice(&parent[..3]),
            2 => {
                mat[0] = unit(mat[0]);
                mat[1] = [0., 1., 0., 0.];
                mat[2] = unit(cross(mat[0], mat[1]));
                mat[1] = unit(cross(mat[2], mat[0]));
            }
            mode => {
                if mode == 1 {
                    mat[2] = parent[2];
                }
                mat[2] = unit(mat[2]);
                mat[1] = unit(cross(mat[2], mat[0]));
                mat[0] = unit(cross(mat[1], mat[2]));
            }
        }
        let mut t = (phase + last as f32) / p.groups[state] as f32;
        if t >= 2. {
            t -= 2.;
        }
        if t >= 1. {
            t -= 1.;
        }
        t = (t + state as f32) * 0.5;
        let elapsed = now.wrapping_sub(self.last_time).max(0);
        self.last_time = now;
        let mut dir = parent;
        if self.deviation_flag > 0 {
            dir[3] = add(mat[3], self.deviation);
        }
        dir[2][1] = 0.;
        dir[2] = unit(dir[2]);
        dir[1] = [0., 1., 0., 0.];
        dir[0] = cross(dir[1], dir[2]);
        for time in &mut self.step_time[..p.count] {
            if *time < p.speed[state] {
                *time = time.wrapping_add(elapsed);
            }
        }
        let mut distance = 0.;
        let mut next = (last + 1) % p.groups[state];
        let mut best = next;
        let mut variables = [[0.; 4]; 8];
        for i in 0..p.count {
            let mut v = sub(point(dir, self.delta[i]), foot[i]);
            v[1] = 0.;
            let f = length(v);
            if f > distance && self.step_time[i] >= p.speed[state] {
                distance = f;
                best = i;
            }
            let mut value = add(foot[i], rotate(dir, offsets[i]));
            value[3] = offsets[i][3];
            variables[i] = value;
        }
        let mut change = usize::from(
            self.step_time[..p.count]
                .iter()
                .all(|&time| time >= p.speed[state]),
        );
        let change_state =
            state == 0 && distance > p.run_high || state == 1 && distance < p.run_low;
        if change_state {
            change += 2;
        }
        if change == 3 && change_state {
            state = 1 - state;
            change = 0;
            last %= p.groups[state];
            self.step_time[..p.count].fill(p.speed[state]);
            next = best % p.groups[state];
        }
        best %= p.groups[state];
        if self.idle && change == 1 {
            next = best;
        }
        self.idle = false;
        if change != 2
            && self.step_time[last] >= p.next[state]
            && self.step_time[next] >= p.speed[state]
        {
            if self.initial_steps > 0 {
                self.initial_steps -= 1;
                distance = 1024.;
            }
            if self.deviation_flag > 0 {
                self.deviation_flag -= 1;
            }
            if distance > p.threshold {
                let v = sub(dir[3], mat[3]);
                let f = length(v);
                if p.flags & 1 != 0 && self.initial_steps == 0 {
                    next = best;
                }
                self.new_position = if f > p.length[state] {
                    add_scaled(mat[3], v, p.length[state] / f)
                } else {
                    dir[3]
                };
                dir[3] = self.new_position;
                let j = self.step_time[last].wrapping_sub(p.next[state]);
                for i in (next..p.count).step_by(p.groups[state]) {
                    self.old[i] = foot[i];
                    let mut v = self.delta[i];
                    v[1] = 0.;
                    self.new[i] = point(dir, v);
                    self.step_time[i] = j;
                }
                last = next;
            } else {
                self.idle = true;
            }
        }
        self.state = state;
        self.last_leg = last;
        self.next_leg = next;
        Ok(Frame {
            transform: mat,
            time: [t, 0., 0., 0.],
            feet: variables,
            count: p.count,
        })
    }
}
fn add(mut a: Vector, b: Vector) -> Vector {
    for i in 0..3 {
        a[i] += b[i];
    }
    a
}
fn sub(mut a: Vector, b: Vector) -> Vector {
    for i in 0..3 {
        a[i] -= b[i];
    }
    a
}
fn scale(mut a: Vector, s: f32) -> Vector {
    for value in &mut a[..3] {
        *value *= s;
    }
    a
}
fn add_scaled(mut a: Vector, b: Vector, s: f32) -> Vector {
    for i in 0..3 {
        a[i] += b[i] * s;
    }
    a
}
fn length(v: Vector) -> f32 {
    f64::from(v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt() as f32
}
fn unit(v: Vector) -> Vector {
    let squared = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
    scale(v, (1. / f64::from(squared).sqrt()) as f32)
}
fn cross(a: Vector, b: Vector) -> Vector {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
        0.,
    ]
}
fn rotate(m: Matrix, v: Vector) -> Vector {
    let mut out = v;
    for i in 0..3 {
        out[i] = m[0][i] * v[0] + m[1][i] * v[1] + m[2][i] * v[2];
    }
    out
}
fn point(m: Matrix, v: Vector) -> Vector {
    let mut out = add(rotate(m, v), m[3]);
    out[3] = 1.;
    out
}
