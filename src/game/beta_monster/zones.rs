//! Released zone enter/leave/collided masks for a particle transition.
use super::hull;
#[derive(Clone, Debug)]
pub struct Zone<'a> {
    pub planes: &'a [[f32; 4]],
    pub enter: u32,
    pub leave: u32,
    pub collided: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct Transition {
    pub old_cell: usize,
    pub new_cell: usize,
    pub start: [f32; 4],
    pub end: [f32; 4],
    pub kind: u32,
    pub shot_select: Option<i32>,
}
pub fn transition(
    zones: &mut [Zone<'_>],
    lists: &[&[usize]],
    q: Transition,
) -> Result<(), &'static str> {
    if q.shot_select.is_some_and(|select| select != 0) {
        return Ok(());
    }
    let mask = 1u32.wrapping_shl(q.kind);
    for (step, id) in [q.old_cell, q.new_cell].into_iter().enumerate() {
        if step == 1 && q.old_cell == q.new_cell {
            break;
        }
        let list = lists.get(id).ok_or("zone cell list out of range")?;
        for &index in *list {
            let zone = zones.get_mut(index).ok_or("zone index out of range")?;
            let before = hull::outside_mask(zone.planes, q.start, 0.) == 0;
            let after = hull::outside_mask(zone.planes, q.end, 0.) == 0;
            if before && !after {
                zone.leave |= mask;
            }
            if !before && after {
                zone.enter |= mask;
            }
            if after {
                zone.collided |= mask;
            }
        }
    }
    Ok(())
}
