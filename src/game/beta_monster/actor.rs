//! Released actor spawn composition. Scene execution and AI registration remain
//! distinct from constructing persistent actor/body/hit-cell storage.
use super::{
    HitBox, InitialState,
    body::Body,
    dynamic::{MovingCell, Pose},
};
use crate::mesh::Matrix;
#[derive(Clone, Debug)]
pub struct Actor {
    pub initial: InitialState,
    pub body: Body,
    pub hit_cell: MovingCell,
}
impl Actor {
    pub fn spawn<F>(
        words: &[u32],
        parent: Matrix,
        spline_token: u32,
        actor_token: u32,
        mut find_cell: F,
    ) -> Result<Self, &'static str>
    where
        F: FnMut([f32; 4]) -> Option<usize>,
    {
        let initial = InitialState::decode(words, parent, spline_token, actor_token)?;
        let hull = HitBox::decode(words)?.collision_hull();
        if hull.count > 6 {
            return Err("dynamic plane capacity exceeded");
        }
        let mut original = [[0.; 4]; 6];
        original[..hull.count].copy_from_slice(&hull.planes[..hull.count]);
        let cell = find_cell(initial.cell_query_position);
        let body = Body::new(
            initial.cell_query_position,
            cell,
            initial.collider_type as i32,
        );
        // Released PrepareDynamic copies planes and matrices but does not apply
        // the matrix yet. First PhysicDCell acceptance publishes world geometry.
        let hit_cell = MovingCell {
            original,
            count: hull.count,
            confirmed: parent,
            current: parent,
            mode: hull.mode | 0x20,
            monster: true,
            pose: Pose {
                planes: original,
                count: hull.count,
                center: hull.center,
                matrix: parent,
                blocked: false,
            },
        };
        Ok(Self {
            initial,
            body,
            hit_cell,
        })
    }
}
