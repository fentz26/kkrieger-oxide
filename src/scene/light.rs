//! Original AddLightJob visibility and CalcSphereBounds projection.
use super::culling::Frustum;
use crate::mesh::Matrix;
pub struct Context {
    camera: super::camera::Environment,
    view: Matrix,
    view_projection: Matrix,
    kkrieger: bool,
}
pub struct Prepared {
    pub operator: usize,
    pub position: [f32; 3],
    pub flags: u32,
    pub color: u32,
    pub amplify: f32,
    pub range: f32,
    pub importance: f32,
    pub rect: [f32; 4],
    pub shadow: Frustum,
    pub zfail: Frustum,
}
impl Prepared {
    pub fn paint_light(&self) -> super::paint::Light {
        super::paint::Light {
            position: self.position,
            range: self.range,
            flags: self.flags,
            shadow_planes: self.shadow.planes[..5].try_into().unwrap(),
            zfail_planes: self.zfail.planes[..self.zfail.count].to_vec(),
        }
    }
}
impl Context {
    pub fn from_camera(
        camera: &super::camera::Environment,
        dimensions: [i32; 2],
        kkrieger: bool,
    ) -> Result<Self, &'static str> {
        let environment = camera.material_environment(dimensions)?;
        Ok(Self {
            camera: camera.clone(),
            view: environment.world_to_camera,
            view_projection: environment.view_projection,
            kkrieger,
        })
    }
    /// Prepare one AddLightJob/InsertLightJob contribution before importance
    /// selection. Its sector clip affects visibility, not projected sphere bounds.
    pub fn prepare(&self, job: &super::exec::Job) -> Result<Option<Prepared>, &'static str> {
        let super::exec::Job::Light {
            operator,
            position,
            flags,
            color,
            amplify,
            range,
            clip,
        } = job
        else {
            return Err("light preparation requires a light job");
        };
        let source = Source {
            position: [position[0], position[1], position[2]],
            range: *range,
            amplify: *amplify,
        };
        let mut frustum = Frustum::from_view_projection(self.view_projection, clip.0);
        frustum.normalize();
        let camera_position = [
            self.camera.space[3][0],
            self.camera.space[3][1],
            self.camera.space[3][2],
        ];
        let Some(visible) = visibility(&source, camera_position, &frustum, self.kkrieger) else {
            return Ok(None);
        };
        let zoom = [self.camera.zoom_x, self.camera.zoom_y];
        let rect = sphere_bounds(source.position, *range, self.view, zoom);
        if rect[2] - rect[0] <= 0. || rect[3] - rect[1] <= 0. {
            return Ok(None);
        }
        let mut shadow = Frustum::from_view_projection(self.view_projection, rect);
        shadow.enlarge_to_include(source.position);
        let zfail = Frustum::zfail_volume(
            source.position,
            self.camera.space,
            self.camera.near_clip,
            zoom,
            rect,
        );
        Ok(Some(Prepared {
            operator: *operator,
            position: source.position,
            flags: *flags,
            color: *color,
            amplify: *amplify * visible.fade,
            range: *range,
            importance: visible.importance,
            rect,
            shadow,
            zfail,
        }))
    }
}
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
