//! Released additive-cell traversal. Subtractive/dynamic clipping is supplied
//! at the terminal boundary; cell links are portable indices.
use super::{add, down, hull, mul};
use crate::collision::Intersection;

#[derive(Clone, Copy, Debug)]
pub struct Ray {
    pub direction: [f32; 3],
    pub end: [f32; 4],
    pub start: [f32; 4],
}
#[derive(Clone, Copy, Debug)]
pub struct AdditiveCell<'a> {
    pub planes: &'a [[f32; 4]],
    pub neighbors: &'a [usize],
    pub only_for_shot: bool,
}
/// Traversal preserves the released visited sequence, including its duplicate
/// initial cell. The callback resolves subtractive/dynamic candidates once.
pub fn cast_additive<F>(
    cells: &[AdditiveCell<'_>],
    current: &mut usize,
    ray: Ray,
    particle_kind: Option<u32>,
    mut sub: F,
) -> Result<Option<Intersection>, &'static str>
where
    F: FnMut(&[usize]) -> Option<Intersection>,
{
    let mut visited = [0; 16];
    visited[0] = *current;
    let mut count = 1;
    let direction = [ray.direction[0], ray.direction[1], ray.direction[2], 0.];
    let player = matches!(particle_kind, Some(1 | 2));
    loop {
        let cell = cells
            .get(*current)
            .ok_or("additive start cell out of range")?;
        let Some(boundary) = hull::intersect_out(cell.planes, ray.end, direction) else {
            return Ok(sub(&visited[..count]));
        };
        if count < visited.len() {
            let mut point = ray.end;
            for (axis, value) in point[..3].iter_mut().enumerate() {
                *value = down(add(
                    f64::from(*value),
                    mul(f64::from(ray.direction[axis]), f64::from(boundary.distance)),
                ));
            }
            let mut next = None;
            for &index in cell.neighbors {
                let neighbor = cells.get(index).ok_or("additive neighbor out of range")?;
                if hull::outside_mask(neighbor.planes, point, 0.) == 0
                    && !(player && neighbor.only_for_shot)
                    && !visited[..count].contains(&index)
                {
                    next = Some(index);
                    break;
                }
            }
            if let Some(next) = next {
                // The released binary appends the cell being left, not the new one.
                visited[count] = *current;
                count += 1;
                *current = next;
                continue;
            }
        }
        return Ok(Some(match sub(&visited[..count]) {
            Some(other) if other.distance < boundary.distance => other,
            _ => boundary,
        }));
    }
}

/// Compose released additive traversal with static/dynamic candidate scanning.
/// The interrupt flag remains caller-owned, as in the original response driver.
/// Geometry errors propagate after any preceding traversal or callback effects.
pub fn cast_world<E: super::subtractive::DynamicEffects>(
    cells: &[AdditiveCell<'_>],
    current: &mut usize,
    world: &mut super::subtractive::CandidateWorld<'_, '_>,
    query: &mut super::subtractive::CandidateQuery,
    interrupted: &mut bool,
    effects: &mut E,
) -> Result<Option<Intersection>, &'static str> {
    let ray = query.ray;
    let kind = query.particle.map(|p| p.kind);
    let mut error = None;
    let result = cast_additive(cells, current, ray, kind, |visited| {
        let mut best = Intersection {
            plane: [0.; 4],
            distance: 0.,
        };
        match world.scan(visited, query, &mut best, interrupted, effects) {
            Ok(true) => Some(best),
            Ok(false) => None,
            Err(reason) => {
                error = Some(reason);
                None
            }
        }
    })?;
    if let Some(error) = error {
        Err(error)
    } else {
        Ok(result)
    }
}
