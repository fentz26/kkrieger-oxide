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

#[derive(Clone, Copy, Debug)]
pub struct ParticleInfo {
    pub kind: u32,
    pub mass: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct FirstParticle {
    pub id: usize,
    pub kind: u32,
    /// None represents a null ShotEvent; otherwise this is its current Select.
    pub shot_select: Option<i32>,
}
#[derive(Clone, Copy, Debug)]
pub struct CandidateQuery {
    pub ray: Ray,
    pub particle: Option<ParticleInfo>,
    pub first: Option<FirstParticle>,
    pub monster_mode: bool,
}
#[derive(Clone, Debug)]
pub struct DynamicCell<'a> {
    pub planes: &'a [[f32; 4]],
    pub mode: u32,
    pub only_for_shot: bool,
    pub monster: bool,
    pub player: bool,
    pub part_box: Option<usize>,
    pub collided: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct ParticleContact {
    pub cell: usize,
    pub particle: usize,
    pub kind: u32,
    pub position: [f32; 4],
    pub plane: [f32; 4],
    pub monster: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct PartBoxContact {
    pub cell: usize,
    pub part_box: usize,
    pub position: [f32; 4],
    pub delta: [f32; 3],
    pub mass: f32,
}
pub trait DynamicEffects {
    fn particle_hit(&mut self, contact: ParticleContact, shot_select: &mut Option<i32>);
    /// The released handler is RET 0x0c. The default preserves that no-op;
    /// an observer can still verify the original call arguments.
    fn part_box_hit(&mut self, _contact: PartBoxContact) {}
}
fn contact_point(ray: Ray, distance: f32) -> [f32; 4] {
    let mut point = ray.end;
    for axis in 0..3 {
        point[axis] = down(add(
            mul(f64::from(ray.direction[axis]), f64::from(distance)),
            f64::from(point[axis]),
        ));
    }
    point
}
pub fn scan_dynamic<E: DynamicEffects>(
    cells: &mut [DynamicCell<'_>],
    query: &mut CandidateQuery,
    best: &mut Intersection,
    interrupted: &mut bool,
    mut accepted: bool,
    effects: &mut E,
) -> bool {
    let non_player = !matches!(query.particle.map(|p| p.kind), Some(1 | 2));
    for (index, cell) in cells.iter_mut().enumerate() {
        if cell.mode != 1
            || (non_player && cell.only_for_shot)
            || ((cell.monster || cell.player) && query.particle.is_none())
        {
            continue;
        }
        match clip(cell.planes, cell.mode, query.monster_mode, query.ray, best) {
            ClipOutcome::Miss => continue,
            ClipOutcome::Inside => *interrupted = true,
            ClipOutcome::Hit => {}
        }
        accepted = true;
        if *interrupted {
            return true;
        }
        if let Some(first) = query.first.as_mut()
            && first.shot_select.is_none_or(|select| select == 0)
        {
            cell.collided |= 1u32.wrapping_shl(first.kind);
            effects.particle_hit(
                ParticleContact {
                    cell: index,
                    particle: first.id,
                    kind: first.kind,
                    position: contact_point(query.ray, best.distance),
                    plane: best.plane,
                    monster: cell.monster,
                },
                &mut first.shot_select,
            );
        }
        if let Some(particle) = query.particle
            && particle.mass > 0.
            && let Some(part_box) = cell.part_box
        {
            let delta = std::array::from_fn(|axis| {
                down(add(
                    f64::from(query.ray.end[axis]),
                    -f64::from(query.ray.start[axis]),
                ))
            });
            effects.part_box_hit(PartBoxContact {
                cell: index,
                part_box,
                position: contact_point(query.ray, best.distance),
                delta,
                mass: particle.mass,
            });
        }
    }
    accepted
}
/// A borrowed view for the complete candidate scan; geometry is immutable,
/// while dynamic collision masks and registry/event state remain mutable.
pub struct CandidateWorld<'a, 'g> {
    pub static_cells: &'a [StaticCell<'g>],
    pub lists: &'a [&'g [usize]],
    pub dynamic_cells: &'a mut [DynamicCell<'g>],
}
impl CandidateWorld<'_, '_> {
    pub fn scan<E: DynamicEffects>(
        &mut self,
        visited: &[usize],
        query: &mut CandidateQuery,
        best: &mut Intersection,
        interrupted: &mut bool,
        effects: &mut E,
    ) -> Result<bool, &'static str> {
        let accepted = scan_static(
            self.static_cells,
            self.lists,
            visited,
            StaticQuery {
                ray: query.ray,
                particle_kind: query.particle.map(|p| p.kind),
                monster_mode: query.monster_mode,
            },
            best,
            interrupted,
        )?;
        if accepted && *interrupted {
            return Ok(true);
        }
        Ok(scan_dynamic(
            self.dynamic_cells,
            query,
            best,
            interrupted,
            accepted,
            effects,
        ))
    }
}
