//! Original AddLightJob visibility and CalcSphereBounds projection.
use super::culling::Frustum;
use crate::mesh::Matrix;
pub struct Source {
    pub position: [f32; 3],
    pub range: f32,
    pub amplify: f32,
}
pub struct Visibility {
    pub importance: f32,
    pub fade: f32,
}
pub fn visibility(
    light: &Source,
    camera_position: [f32; 3],
    frustum: &Frustum,
    kkrieger: bool,
) -> Option<Visibility> {
    for plane in &frustum.planes[..frustum.count] {
        if plane.distance(light.position) < -light.range {
            return None;
        }
    }
    let delta: [f32; 3] = std::array::from_fn(|i| light.position[i] - camera_position[i]);
    let squared = delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2];
    let distance = f64::from(squared).sqrt() as f32;
    let importance = light.range / if distance > 0.1 { distance } else { 0.1 };
    let fade = if kkrieger {
        ((45. - distance) / 10.).clamp(0., 1.)
    } else {
        1.
    };
    if fade > 0. && light.amplify >= 1. / 255. {
        Some(Visibility { importance, fade })
    } else {
        None
    }
}
pub fn sphere_bounds(center: [f32; 3], radius: f32, view: Matrix, zoom: [f32; 2]) -> [f32; 4] {
    let c: [f32; 3] = std::array::from_fn(|axis| {
        view[0][axis] * center[0]
            + view[1][axis] * center[1]
            + view[2][axis] * center[2]
            + view[3][axis]
    });
    let mut rect = [-1., -1., 1., 1.];
    for i in 0..2 {
        let ci = c[i];
        let ds = ci * ci + c[2] * c[2];
        let a = ds - radius * radius;
        if a > 0. {
            let a = f64::from(a).sqrt() as f32;
            let sds = ci * a - c[2] * radius;
            let cds = ci * radius + c[2] * a;
            if c[2] * ds > -radius * sds {
                let value = sds / cds * zoom[i];
                rect[i] = if -1. > value { -1. } else { value };
            }
            let sds = c[2] * radius + ci * a;
            let cds = c[2] * a - ci * radius;
            if c[2] * ds > radius * sds {
                let value = sds / cds * zoom[i];
                rect[i + 2] = if 1. < value { 1. } else { value };
            }
        }
    }
    rect
}
