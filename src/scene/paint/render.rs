//! RenderPaintJobs transitions and the original Material11Insert hooks.
use super::{DrawKind, FramePlan};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    ClearScissor,
    ClearDestinationAlpha,
    AddDestinationAlpha,
    ResetShadowStates,
    ClearStencil,
    ApplyViewProjection,
    GrabCurrentRender,
    SetLight(usize),
    /// Index in FramePlan.jobs, preserving its stable sorted order.
    Draw(usize),
}
#[derive(Clone, Copy, PartialEq, Eq)]
struct Group {
    pass: usize,
    usage: u32,
    light: Option<usize>,
}
fn after(actions: &mut Vec<Action>, usage: u32, ps11: bool) {
    use Action::*;
    match usage {
        0 => {
            if ps11 {
                actions.push(ClearDestinationAlpha);
            }
            actions.push(ApplyViewProjection);
        }
        3 => actions.push(ResetShadowStates),
        4 => actions.extend([ClearStencil, ClearScissor]),
        5 if ps11 => actions.extend([AddDestinationAlpha, ApplyViewProjection]),
        _ => {}
    }
}
/// Translate a sorted paint plan into draw/state operations. Current-render
/// requests come from effect cache metadata. Inserts here are Material11;
/// Material20 requires its separate original hook implementation.
pub fn actions(frame: &FramePlan, current_render: &[bool; 256], ps11: bool) -> Vec<Action> {
    use Action::*;
    let mut inserts = [false; 256];
    for insert in &frame.inserts {
        inserts[insert.insert.render_pass as usize] = true;
    }
    let mut actions = Vec::new();
    let mut previous: Option<Group> = None;
    for (index, job) in frame.jobs.iter().enumerate() {
        let group = Group {
            pass: (job.sort_key >> 24) as usize,
            usage: (job.sort_key >> 16) & 15,
            light: match &job.kind {
                DrawKind::Mesh(mesh) => mesh.paint.light,
                DrawKind::Effect => None,
            },
        };
        if previous != Some(group) {
            actions.push(ClearScissor);
            if let Some(old) = previous
                && inserts[old.pass]
            {
                after(&mut actions, old.usage, ps11);
            }
            if previous.is_none_or(|old| old.pass != group.pass) && current_render[group.pass] {
                actions.push(GrabCurrentRender);
            }
            if inserts[group.pass] && group.usage == 5 {
                actions.push(ClearScissor);
            }
            if let Some(light) = group.light {
                actions.push(SetLight(light));
            }
            previous = Some(group);
        }
        actions.push(Draw(index));
        if matches!(job.kind, DrawKind::Effect) {
            actions.push(ApplyViewProjection);
        }
    }
    actions.push(ClearScissor);
    if let Some(old) = previous
        && inserts[old.pass]
    {
        after(&mut actions, old.usage, ps11);
    }
    actions
}
