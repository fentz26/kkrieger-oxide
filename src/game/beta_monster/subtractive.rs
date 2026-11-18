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
pub fn clip(
    planes: &[[f32; 4]],
    mode: u32,
    monster_mode: bool,
    ray: Ray,
    best: &mut Intersection,
) -> ClipOutcome {
    if mode != 1 && !(mode == 9 && monster_mode) {
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
