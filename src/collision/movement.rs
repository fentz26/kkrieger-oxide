//! Original MoveCollider and recursive soft-sphere world traversal.
use super::{Cell, DynamicCell, ORDER, ray::Info, sphere, world::World};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Debug)]
pub struct Collider {
    pub position: [f32; 4],
    pub radius: f32,
    pub cell: usize,
}

fn sub(sphere: &mut [f32; 4], cell: &Cell) {
    for order in ORDER {
        sphere::collide_face(sphere, &order.map(|i| cell.vertices[i]));
    }
}

impl World {
    fn soft_sphere_add(&self, sphere: &mut [f32; 4], visited: &mut [usize; 16], count: &mut usize) {
        let index = visited[*count - 1];
        let cell = &self.entries[index].cell;
        if cell.approximate_distance(*sphere).abs() < sphere[3]
            && let Some(faces) = &self.faces[index]
        {
            for face in faces {
                sphere::collide_face(sphere, face);
            }
        }
        for &neighbor in &self.connectivity.neighbors[index].adds {
            if self.entries[neighbor].cell.approximate_distance(*sphere) < sphere[3]
                && !visited[..*count].contains(&neighbor)
                && *count < 16
            {
                visited[*count] = neighbor;
                *count += 1;
                self.soft_sphere_add(sphere, visited, count);
            }
        }
        for &neighbor in &self.connectivity.neighbors[index].subs {
            let cell = &self.entries[neighbor].cell;
            if cell.approximate_distance(*sphere) < sphere[3] {
                sub(sphere, cell);
            }
        }
    }

    /// `who` uses the original KCRF bits: player=1, monster=2. The player's
    /// dynamic hit cell is skipped by pointer identity during sphere response.
    pub fn move_collider(
        &mut self,
        collider: &mut Collider,
        movement: [f32; 4],
        who: u32,
        dynamic: &[Rc<RefCell<DynamicCell>>],
        player_hit: Option<&Rc<RefCell<DynamicCell>>>,
    ) -> Result<bool, &'static str> {
        if self.faces.get(collider.cell).is_none_or(Option::is_none) {
            return Err("movement start cell is not additive");
        }
        let old = collider.position;
        for (value, delta) in collider.position[..3].iter_mut().zip(movement) {
            *value += delta;
        }
        let mut sphere = collider.position;
        sphere[3] = collider.radius;
        let mut visited = [0; 16];
        visited[0] = collider.cell;
        let mut count = 1;
        self.soft_sphere_add(&mut sphere, &mut visited, &mut count);
        for handle in dynamic {
            let cell = handle.borrow();
            if who & 2 != 0 && cell.monster.is_some() {
                continue;
            }
            if who & 1 != 0 && player_hit.is_some_and(|player| Rc::ptr_eq(player, handle)) {
                continue;
            }
            sub(&mut sphere, &cell.cell);
        }
        collider.position = sphere;
        collider.position[3] = 1.;
        let mut info = Info {
            zone_mask: if who & 1 != 0 { 2 } else { 0 },
            flags: who,
            ..Default::default()
        };
        self.collide_ray(
            &mut collider.cell,
            collider.position,
            old,
            &mut info,
            dynamic,
        )?;
        if self.entries[collider.cell]
            .cell
            .outside_mask(collider.position, 0.)
            != 0
            && let Some(cell) = self.find_cell(collider.position)
        {
            collider.cell = cell;
        }
        // The source returns false even when the position was corrected.
        Ok(false)
    }
}
