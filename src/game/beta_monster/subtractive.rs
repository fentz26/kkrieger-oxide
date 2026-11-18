//! Released single-subtractive-cell clipping (CollideRaySub2).
use super::{add, collision::SEPARATION, down, hull, mul, world_cast::Ray};
use crate::collision::Intersection;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipOutcome {
    Miss,
    Hit,
    /// The caller sets the shared interrupt flag and stops candidate scanning.
    Inside,
}
fn eligible(mode: u32, monster_mode: bool) -> bool {
    mode == 1 || (mode == 9 && monster_mode)
}
pub fn clip(
    planes: &[[f32; 4]],
    mode: u32,
    monster_mode: bool,
    ray: Ray,
    best: &mut Intersection,
) -> ClipOutcome {
    if !eligible(mode, monster_mode) {
        return ClipOutcome::Miss;
    }
    let start_mask = hull::outside_mask(planes, ray.start, 0.);
    let end_mask = hull::outside_mask(planes, ray.end, 0.);
    if start_mask == 0 {
        *best = Intersection {
            plane: [1., 0., 0., 0.],
            distance: 0.,
        };
        return ClipOutcome::Inside;
    }
    if start_mask & end_mask != 0 {
        return ClipOutcome::Miss;
    }
    let direction = [ray.direction[0], ray.direction[1], ray.direction[2], 0.];
    let Some(entry) = hull::intersect_in(planes, ray.end, direction) else {
        return ClipOutcome::Miss;
    };
    if entry.distance.partial_cmp(&best.distance) != Some(std::cmp::Ordering::Less) {
        return ClipOutcome::Miss;
    }
    let mut point = ray.end;
    for axis in 0..3 {
        point[axis] = down(add(
            mul(f64::from(ray.direction[axis]), f64::from(entry.distance)),
            f64::from(point[axis]),
        ));
    }
    if hull::outside_mask(planes, point, SEPARATION) != 0 {
        return ClipOutcome::Miss;
    }
    *best = Intersection {
        plane: entry.plane.map(|v| -v),
        distance: entry.distance,
    };
    ClipOutcome::Hit
}

#[derive(Clone, Copy, Debug)]
pub struct StaticCell<'a> {
    pub planes: &'a [[f32; 4]],
    pub mode: u32,
    pub only_for_shot: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct StaticQuery {
    pub ray: Ray,
    pub particle_kind: Option<u32>,
    pub monster_mode: bool,
}
/// Begin CollideRaySub's static phase. Distance is reset even for an empty scan;
/// plane and interrupt state persist until the original operation changes them.
pub fn scan_static(
    cells: &[StaticCell<'_>],
    lists: &[&[usize]],
    visited: &[usize],
    query: StaticQuery,
    best: &mut Intersection,
    interrupted: &mut bool,
) -> Result<bool, &'static str> {
    best.distance = SEPARATION;
    let non_player = !matches!(query.particle_kind, Some(1 | 2));
    let mut accepted = false;
    for &add in visited {
        let list = lists.get(add).ok_or("static additive list out of range")?;
        for &index in *list {
            let cell = cells
                .get(index)
                .ok_or("static subtractive cell out of range")?;
            if !eligible(cell.mode, query.monster_mode) || (non_player && cell.only_for_shot) {
                continue;
            }
            match clip(cell.planes, cell.mode, query.monster_mode, query.ray, best) {
                ClipOutcome::Miss => {}
                ClipOutcome::Inside => {
                    *interrupted = true;
                    return Ok(true);
                }
                ClipOutcome::Hit => {
                    accepted = true;
                    if *interrupted {
                        return Ok(true);
                    }
                }
            }
        }
    }
    Ok(accepted)
}
