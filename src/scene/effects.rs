//! Original effect BuildPaintJobs keys and stable SortPaintJobs order.
use super::{Error, exec::Job};
use crate::kx::Document;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scheduled {
    /// Index in the scene's original job array, retaining its animation snapshot.
    pub job: usize,
    pub sort_key: u32,
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
        let op = doc.operators.get(*operator).ok_or(Error {
            operator: *operator,
            reason: "effect job operator out of range",
        })?;
        let class = doc.classes[op.class].id;
        if !matches!(class, 0x61 | 0x64) {
            continue;
        }
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
