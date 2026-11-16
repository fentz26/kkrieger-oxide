//! Original mesh material-pass expansion within Engine::BuildPaintJobs.
use super::culling::{Bounds, Plane};
use crate::mesh::Matrix;
pub struct Candidate {
    pub bounds: Bounds,
    /// Final world/bone matrix selected by the mesh animation stage.
    pub matrix: Matrix,
    pub animated: bool,
    pub usage: u32,
    pub render_pass: i32,
    pub mesh_pass: i32,
    pub pass_adjust: i32,
    /// Low 16 bits of the renderer's material-pass sorting identity.
    pub material_key: u16,
}
pub struct Light {
    pub position: [f32; 3],
    pub range: f32,
    pub flags: u32,
    pub shadow_planes: [Plane; 5],
    pub zfail_planes: Vec<Plane>,
}
pub struct View<'a> {
    pub usage_mask: u32,
    pub frustum: [Plane; 5],
    pub lights: &'a [Light],
    /// Original KKRIEGER build uses four lights; other builds use sixteen.
    pub light_capacity: usize,
}
#[derive(Debug)]
pub struct Paint {
    pub sort_key: u32,
    pub light: Option<usize>,
    pub zfail: bool,
}
pub struct Built {
    /// Insert selection happens here even when all per-light jobs are culled.
    pub render_pass: Option<u8>,
    pub jobs: Vec<Paint>,
}
pub fn build(candidate: &Candidate, view: &View<'_>) -> Result<Built, &'static str> {
    if !matches!(view.light_capacity, 4 | 16) || view.lights.len() > view.light_capacity {
        return Err("invalid original paint light capacity");
    }
    if candidate.usage >= 32 {
        return Err("paint usage exceeds original usage mask");
    }
    let empty = || Built {
        render_pass: None,
        jobs: Vec::new(),
    };
    if view.usage_mask & (1 << candidate.usage) == 0 {
        return Ok(empty());
    }
    let bounds = candidate.bounds.transformed(candidate.matrix);
    let culled = !candidate.animated && bounds.culled(&view.frustum);
    if culled && candidate.usage != 3 {
        return Ok(empty());
    }
    let pass = candidate.render_pass.wrapping_add(candidate.mesh_pass) & 255;
    let pass = pass.wrapping_add(candidate.pass_adjust).clamp(0, 255) as u32;
    let key = |light: usize| {
        (pass << 24)
            | ((light as u32) << 20)
            | (candidate.usage << 16)
            | u32::from(candidate.material_key)
    };
    let mut jobs = Vec::new();
    if (3..=4).contains(&candidate.usage) {
        for (index, light) in view.lights.iter().enumerate() {
            if !bounds.intersects_sphere(light.position, light.range) {
                continue;
            }
            if candidate.usage == 3 && (light.flags & 2 == 0 || bounds.culled(&light.shadow_planes))
            {
                continue;
            }
            jobs.push(Paint {
                sort_key: key(index),
                light: Some(index),
                zfail: !bounds.culled(&light.zfail_planes),
            });
        }
    } else {
        let light = if candidate.usage < 3 {
            0
        } else {
            view.light_capacity - 1
        };
        jobs.push(Paint {
            sort_key: key(light),
            light: None,
            zfail: false,
        });
    }
    Ok(Built {
        render_pass: Some(pass as u8),
        jobs,
    })
}
