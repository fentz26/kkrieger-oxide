//! Original mesh material-pass expansion within Engine::BuildPaintJobs.
use super::culling::{Bounds, Plane};
use crate::mesh::Matrix;
use std::rc::Rc;

pub struct Asset {
    pub mesh: Rc<crate::prepared_mesh::PreparedMesh>,
    pub materials: Vec<Rc<crate::material::Material>>,
}
pub struct Assets {
    /// Keyed by canonical mesh cache identity, including aliased root requests.
    pub meshes: std::collections::BTreeMap<usize, Asset>,
    pub materials: std::collections::BTreeMap<usize, Rc<crate::material::Material>>,
    pub default_material: Rc<crate::material::Material>,
}
pub fn prepare_assets(
    doc: &crate::kx::Document<'_>,
    roots: &[usize],
    intro: bool,
) -> Result<Assets, Box<dyn std::error::Error>> {
    prepare_assets_with_materials(doc, roots, &[], intro)
}
pub fn prepare_assets_with_materials(
    doc: &crate::kx::Document<'_>,
    roots: &[usize],
    extra_material_roots: &[usize],
    intro: bool,
) -> Result<Assets, Box<dyn std::error::Error>> {
    use crate::mesh::MaterialSource;
    use std::collections::{BTreeMap, BTreeSet};
    let mut owners = BTreeSet::new();
    for &root in roots {
        let owner =
            super::cache_mesh_identity(doc, root)?.ok_or("paint root does not cache a mesh")?;
        owners.insert(owner);
    }
    let owners: Vec<_> = owners.into_iter().collect();
    let prepared =
        crate::prepared_mesh::generate_many_with_materials(doc, &owners, extra_material_roots)?;
    let default_material = crate::material::original_default(intro);
    let null_material = Rc::new(crate::material::Material::default());
    let mut meshes = BTreeMap::new();
    for (owner, mesh) in owners.into_iter().zip(prepared.meshes) {
        let mut materials = Vec::with_capacity(mesh.materials.len());
        for slot in &mesh.materials {
            materials.push(match slot.source {
                MaterialSource::Null => Rc::clone(&null_material),
                MaterialSource::Default => Rc::clone(&default_material),
                MaterialSource::Operator(index) => prepared
                    .materials
                    .get(&index)
                    .cloned()
                    .ok_or("prepared paint material is unavailable")?,
            });
        }
        meshes.insert(owner, Asset { mesh, materials });
    }
    Ok(Assets {
        meshes,
        materials: prepared.materials,
        default_material,
    })
}
pub enum DrawKind {
    Effect,
    Mesh(MeshPaint),
}
pub struct Queued {
    pub scene_job: usize,
    pub sort_key: u32,
    pub kind: DrawKind,
}
pub struct FrameInsert {
    pub scene_job: usize,
    pub insert: InsertUse,
}
pub struct FramePlan {
    pub jobs: Vec<Queued>,
    pub inserts: Vec<FrameInsert>,
}
/// Original BuildPaintJobs/SortPaintJobs ordering for prepared static meshes
/// and deferred effects. Geometry and effect execution remain at draw time.
pub fn build_static_frame(
    doc: &crate::kx::Document<'_>,
    jobs: &[super::exec::Job],
    assets: &std::collections::BTreeMap<usize, Asset>,
    materials: &std::collections::BTreeMap<usize, Rc<crate::material::Material>>,
    view: &View<'_>,
    mut material_key: impl FnMut(&Rc<crate::material::Material>, usize) -> u16,
) -> Result<FramePlan, super::Error> {
    let effects = super::effects::schedule_prepared(doc, jobs, materials)?;
    let mut frame = FramePlan {
        jobs: effects
            .into_iter()
            .map(|effect| Queued {
                scene_job: effect.job,
                sort_key: effect.sort_key,
                kind: DrawKind::Effect,
            })
            .collect(),
        inserts: Vec::new(),
    };
    // MeshJob, like EffectJob, is prepended by AddPaintJob. Effects are added
    // to the final array first, so equal-key ties keep effects before meshes.
    for (scene_job, job) in jobs.iter().enumerate().rev() {
        let super::exec::Job::Paint {
            operator,
            matrix,
            pass_adjust,
            ..
        } = job
        else {
            continue;
        };
        let fail = |reason| super::Error {
            operator: *operator,
            reason,
        };
        let op = doc
            .operators
            .get(*operator)
            .ok_or_else(|| fail("paint job operator out of range"))?;
        let id = doc.classes[op.class].id;
        if matches!(id, 0x61 | 0x64) {
            continue;
        }
        if !crate::mesh_graph::implemented(id) {
            return Err(fail("paint job cache type has not been ported"));
        }
        let asset = assets
            .get(operator)
            .ok_or_else(|| fail("mesh paint asset has not been prepared"))?;
        let instance = StaticInstance {
            mesh: &asset.mesh,
            materials: &asset.materials,
            matrix: *matrix,
            pass_adjust: *pass_adjust,
        };
        let built = build_static(&instance, view, &mut material_key).map_err(fail)?;
        frame.inserts.extend(
            built
                .inserts
                .into_iter()
                .map(|insert| FrameInsert { scene_job, insert }),
        );
        frame
            .jobs
            .extend(built.jobs.into_iter().map(|paint| Queued {
                scene_job,
                sort_key: paint.paint.sort_key,
                kind: DrawKind::Mesh(paint),
            }));
    }
    frame.jobs.sort_by_key(|job| job.sort_key);
    Ok(frame)
}

pub struct StaticInstance<'a> {
    pub mesh: &'a crate::prepared_mesh::PreparedMesh,
    /// Resolved original materials, including the unused slot zero.
    pub materials: &'a [Rc<crate::material::Material>],
    pub matrix: Matrix,
    pub pass_adjust: i32,
}
pub struct MeshPaint {
    pub material_slot: usize,
    pub pass: usize,
    pub geometry_job: usize,
    pub paint: Paint,
}
pub struct InsertUse {
    pub material_slot: usize,
    pub render_pass: u8,
}
pub struct StaticPaint {
    pub jobs: Vec<MeshPaint>,
    pub inserts: Vec<InsertUse>,
}
/// Expand a non-bone-animated mesh in original material/pass order. Material
/// keys must resolve existing pass identities, including shared allocations.
pub fn build_static(
    instance: &StaticInstance<'_>,
    view: &View<'_>,
    mut material_key: impl FnMut(&Rc<crate::material::Material>, usize) -> u16,
) -> Result<StaticPaint, &'static str> {
    let mesh = instance.mesh;
    if instance.materials.len() != mesh.materials.len()
        || mesh.pass_jobs.len() != mesh.materials.len()
    {
        return Err("paint material slot count mismatch");
    }
    let mut output = StaticPaint {
        jobs: Vec::new(),
        inserts: Vec::new(),
    };
    for (slot, material) in instance.materials.iter().enumerate().skip(1) {
        if mesh.pass_jobs[slot].len() != material.passes.len() {
            return Err("paint pass-to-geometry mapping mismatch");
        }
        for (pass_index, pass) in material.passes.iter().enumerate() {
            let geometry_job = mesh.pass_jobs[slot][pass_index];
            let geometry = mesh
                .jobs
                .get(geometry_job)
                .ok_or("paint geometry job out of range")?;
            if geometry.material != slot || geometry.program != pass.program {
                return Err("paint geometry material or program mismatch");
            }
            let candidate = Candidate {
                bounds: Bounds(geometry.bounds),
                matrix: instance.matrix,
                animated: false,
                usage: pass.usage.as_raw(),
                render_pass: pass.render_pass as i32,
                mesh_pass: mesh.materials[slot].pass,
                pass_adjust: instance.pass_adjust,
                material_key: material_key(material, pass_index),
            };
            let built = build(&candidate, view)?;
            if material.material11_insert
                && let Some(render_pass) = built.render_pass
            {
                output.inserts.push(InsertUse {
                    material_slot: slot,
                    render_pass,
                });
            }
            output
                .jobs
                .extend(built.jobs.into_iter().map(|paint| MeshPaint {
                    material_slot: slot,
                    pass: pass_index,
                    geometry_job,
                    paint,
                }));
        }
    }
    Ok(output)
}
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
