//! Original Scene_Particles cached positions and per-instance transforms.
use crate::{
    kx::Spline,
    mesh::{Matrix, identity},
    random::Random,
};
#[derive(Clone, Copy, PartialEq)]
pub struct Parameters {
    pub mode: i32,
    pub count: i32,
    pub seed: i32,
    pub spread: [f32; 3],
}
#[derive(Default)]
pub struct State {
    parameters: Option<Parameters>,
    pub positions: Vec<[f32; 4]>,
}
impl State {
    pub fn matrices(
        &mut self,
        parameters: Parameters,
        animation: f32,
        line: [f32; 3],
        spline: Option<&Spline>,
        random: &mut Random,
    ) -> Result<Vec<Matrix>, &'static str> {
        if self.parameters != Some(parameters) {
            let count =
                usize::try_from(parameters.count).map_err(|_| "negative scene particle count")?;
            self.positions.clear();
            self.positions
                .try_reserve(count)
                .map_err(|_| "scene particle allocation failed")?;
            *random = Random::new(parameters.seed);
            for index in 0..count {
                let mut position: [f32; 4];
                loop {
                    position = [
                        random.float() * 2. - 1.,
                        random.float() * 2. - 1.,
                        random.float() * 2. - 1.,
                        0.,
                    ];
                    let squared = position[0] * position[0]
                        + position[1] * position[1]
                        + position[2] * position[2];
                    if parameters.mode & 12 == 4 {
                        let scale = (1. / f64::from(squared).sqrt()) as f32;
                        for value in &mut position[..3] {
                            *value *= scale;
                        }
                    }
                    if parameters.mode & 12 != 8 || squared <= 1. {
                        break;
                    }
                }
                position[3] = if parameters.mode & 0x20 != 0 {
                    index as f32 / parameters.count as f32
                } else {
                    random.float()
                };
                for (value, spread) in position[..3].iter_mut().zip(parameters.spread) {
                    *value *= spread * 0.5;
                }
                self.positions.push(position);
            }
            self.parameters = Some(parameters);
        }
        let mut animation = animation % 1.;
        if animation < 0. {
            animation += 1.;
        }
        Ok(self
            .positions
            .iter()
            .map(|position| {
                let mut matrix = identity();
                let mut time = animation + position[3];
                if time >= 1. {
                    time -= 1.;
                }
                let offset = if parameters.mode & 1 != 0
                    && let Some(spline) = spline
                {
                    let point = spline.evaluate(time);
                    if parameters.mode & 0x10 != 0 {
                        let next = spline.evaluate(time + 0.01);
                        matrix = direction(std::array::from_fn(|i| next[i] - point[i]));
                    }
                    [point[0], point[1], point[2]]
                } else {
                    line.map(|value| value * time)
                };
                for axis in 0..3 {
                    matrix[3][axis] += position[axis] + offset[axis];
                }
                matrix[3][3] = 1.;
                matrix
            })
            .collect())
    }
}
fn direction(v: [f32; 3]) -> Matrix {
    fn unit(v: [f32; 3]) -> [f32; 3] {
        let squared = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
        if squared > 1e-20 {
            let scale = (1. / f64::from(squared).sqrt()) as f32;
            v.map(|value| value * scale)
        } else {
            [1., 0., 0.]
        }
    }
    fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    }
    let k = unit(v);
    let i = unit(cross([0., 1., 0.], k));
    let j = unit(cross(k, i));
    [
        [i[0], i[1], i[2], 0.],
        [j[0], j[1], j[2], 0.],
        [k[0], k[1], k[2], 0.],
        [0., 0., 0., 1.],
    ]
}
