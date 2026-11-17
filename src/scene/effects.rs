//! Original effect BuildPaintJobs keys and stable SortPaintJobs order.
use super::{Error, exec::Job};
use crate::kx::Document;

pub enum Content {
    Print(Box<super::print::Draw>),
    Particles(Vec<[[f32; 8]; 4]>),
}
pub struct Rendering {
    pub camera: super::camera::Environment,
    pub materials: std::collections::BTreeMap<usize, std::rc::Rc<crate::material::Material>>,
    pub pages: std::collections::BTreeMap<u8, crate::bitmap::FontPage>,
    pub fps: f32,
}
pub struct Leaf {
    pub operator: usize,
    pub material: Option<usize>,
    pub content: Content,
}
pub struct Draw {
    pub operator: usize,
    pub sort_key: u32,
    pub material: Option<usize>,
    pub content: Content,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scheduled {
    /// Index in the scene's original job array, retaining its animation snapshot.
    pub job: usize,
    pub sort_key: u32,
}

/// Geometry produced at a deferred particle effect's position in paint order.
pub struct ParticleDraw {
    pub operator: usize,
    pub sort_key: u32,
    pub material: Option<usize>,
    pub quads: Vec<[[f32; 8]; 4]>,
}

/// Resolve a linked cached material to its first pass and raw usage. A missing
/// material or empty pass list returns None. Mesh jobs are scheduled separately
/// by the engine; these keys allow a stable merge with that paint-job array.
pub fn schedule(
    doc: &Document<'_>,
    jobs: &[Job],
    mut material: impl FnMut(usize) -> Option<(u32, u32)>,
) -> Result<Vec<Scheduled>, Error> {
    let mut scheduled = Vec::new();
    // AddPaintJob prepends to EffectJobs; BuildPaintJobs walks that linked list.
    for (job, entry) in jobs.iter().enumerate().rev() {
        let Job::Paint {
            operator,
            pass_adjust,
            ..
        } = entry
        else {
            continue;
        };
        let super::CachedObject::Effect(owner) = super::cached_object(doc, *operator)? else {
            continue;
        };
        let op = &doc.operators[owner];
        let class = doc.classes[op.class].id;
        let first = op.links.first().copied().flatten().and_then(&mut material);
        let (pass, usage) = match (class, first) {
            (0x64, Some((pass, _))) => (pass, 7),
            (_, Some(first)) => first,
            (_, None) => (0, 7),
        };
        let pass = (pass as i32).wrapping_add(*pass_adjust).clamp(0, 255) as u32;
        scheduled.push(Scheduled {
            job,
            sort_key: (pass << 24) | (usage << 16),
        });
    }
    scheduled.sort_by_key(|entry| entry.sort_key);
    Ok(scheduled)
}

/// Resolve and validate prepared material metadata before executing any effect.
pub fn schedule_prepared(
    doc: &Document<'_>,
    jobs: &[Job],
    materials: &std::collections::BTreeMap<usize, std::rc::Rc<crate::material::Material>>,
) -> Result<Vec<Scheduled>, Error> {
    let scheduled = schedule(doc, jobs, |index| {
        let first = materials.get(&index)?.passes.first()?;
        let usage = first.usage.as_raw();
        Some((first.render_pass, usage))
    })?;
    // A linked material absent from the prepared cache is an integration
    // error, distinct from an authored null material or an empty pass list.
    for entry in &scheduled {
        let Job::Paint { operator, .. } = &jobs[entry.job] else {
            unreachable!()
        };
        let super::CachedObject::Effect(owner) = super::cached_object(doc, *operator)? else {
            unreachable!()
        };
        if let Some(material) = doc.operators[owner].links.first().copied().flatten()
            && !materials.contains_key(&material)
        {
            return Err(Error {
                operator: *operator,
                reason: "effect material has not been prepared",
            });
        }
    }
    Ok(scheduled)
}
