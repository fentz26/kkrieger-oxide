//! Original CollideRay/CollideRaySub/CollideRaySub2, including zone side effects.
use super::{Cell, DynamicCell, world::World};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Debug, Default)]
pub struct Info {
    pub plane: [f32; 4],
    pub position: [f32; 4],
    pub distance: f32,
    pub radius: f32,
    pub zone_mask: u32,
    pub flags: u32,
    pub dynamic_cell: Option<Rc<RefCell<DynamicCell>>>,
}

fn position(point: [f32; 4], direction: [f32; 4], distance: f32) -> [f32; 4] {
    let mut result = point;
    for i in 0..3 {
        result[i] += direction[i] * distance;
    }
    result
}

fn sub2(cell: &Cell, direction: [f32; 4], end: [f32; 4], start: [f32; 4], info: &mut Info) -> bool {
    if cell.reject_plane(start, end) {
        return false;
    }
    if let Some(hit) = cell.intersect_in(end, direction, info.radius)
        && hit.distance < info.distance
    {
        let point = position(end, direction, hit.distance);
        if cell.outside_mask(point, info.radius - 4.010e-3) == 0 {
            info.distance = hit.distance;
            info.plane = hit.plane.map(|x| -x);
            info.position = point;
            return true;
        }
    }
    false
}

fn zone(cell: &mut Cell, start: [f32; 4], end: [f32; 4], mask: u32) -> bool {
    let hit0 = cell.outside_mask(start, 0.) == 0;
    let hit1 = cell.outside_mask(end, 0.) == 0;
    if hit0 && !hit1 {
        cell.interactions.leave |= mask;
    }
    if !hit0 && hit1 {
        cell.interactions.enter |= mask;
    }
    if hit1 {
        cell.interactions.collided |= mask;
    }
    hit1
}

impl World {
    /// `add` is updated while traversing connected additive cells. The return
    /// value is separate from Info: zones can assign DCell without a collision.
    pub fn collide_ray(
        &mut self,
        add: &mut usize,
        end: [f32; 4],
        start: [f32; 4],
        info: &mut Info,
        dynamic: &[Rc<RefCell<DynamicCell>>],
    ) -> Result<bool, &'static str> {
        if self.faces.get(*add).is_none_or(Option::is_none) {
            return Err("ray start cell is not additive");
        }
        let direction = std::array::from_fn(|i| end[i] - start[i]);
        let mut visited = [0; 16];
        visited[0] = *add;
        let mut count = 1;
        loop {
            if let Some(hit) = self.entries[*add].cell.intersect_out(end, direction, 0.) {
                info.plane = hit.plane;
                info.distance = hit.distance;
                let point = position(end, direction, hit.distance);
                info.position = point;
                let next = if count < 16 {
                    self.connectivity.neighbors[*add]
                        .adds
                        .iter()
                        .copied()
                        .find(|&i| {
                            self.entries[i].cell.outside_mask(point, 0.) == 0
                                && !visited[..count].contains(&i)
                        })
                } else {
                    None
                };
                if let Some(next) = next {
                    *add = next;
                    visited[count] = next;
                    count += 1;
                    continue;
                }
                let mut other = info.clone();
                if self.collide_ray_sub(
                    direction,
                    end,
                    start,
                    &visited[..count],
                    &mut other,
                    dynamic,
                ) && other.distance < info.distance
                {
                    info.distance = other.distance;
                    info.plane = other.plane;
                    info.position = other.position;
                    // Source copies only these fields, retaining DCell.
                }
                return Ok(true);
            }
            return Ok(self.collide_ray_sub(
                direction,
                end,
                start,
                &visited[..count],
                info,
                dynamic,
            ));
        }
    }

    fn collide_ray_sub(
        &mut self,
        direction: [f32; 4],
        end: [f32; 4],
        start: [f32; 4],
        visited: &[usize],
        info: &mut Info,
        dynamic: &[Rc<RefCell<DynamicCell>>],
    ) -> bool {
        let mut collided = false;
        info.distance = 4.010e-3;
        for &add in visited {
            for &i in &self.connectivity.neighbors[add].subs {
                let cell = &self.entries[i].cell;
                if cell.mode & 3 == 1 && sub2(cell, direction, end, start, info) {
                    collided = true;
                    info.dynamic_cell = None;
                }
            }
            for &i in &self.connectivity.neighbors[add].zones {
                zone(&mut self.entries[i].cell, start, end, info.zone_mask);
            }
        }
        for handle in dynamic {
            let mut dynamic = handle.borrow_mut();
            let cell = &mut dynamic.cell;
            if cell.mode & 3 == 1 && sub2(cell, direction, end, start, info) {
                collided = true;
                info.dynamic_cell = Some(Rc::clone(handle));
            }
            if cell.mode & 3 == 2 && zone(cell, start, end, info.zone_mask) {
                info.dynamic_cell = Some(Rc::clone(handle));
            }
        }
        collided
    }
}
