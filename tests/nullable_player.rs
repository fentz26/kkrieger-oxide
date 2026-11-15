use kkrieger_oxide::{
    collision::{connectivity, world::World},
    game::player::Motion,
};
use std::rc::Rc;
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let v = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        v
    }
    fn float(&mut self) -> f32 {
        f32::from_bits(self.word())
    }
    fn vector(&mut self) -> [f32; 4] {
        std::array::from_fn(|_| self.float())
    }
    fn check_float(&mut self, v: f32) {
        assert_eq!(v.to_bits(), self.word(), "{v}");
    }
    fn check_vector(&mut self, v: [f32; 4]) {
        for x in v {
            self.check_float(x);
        }
    }
    fn placement(&mut self, m: &Motion) {
        self.check_vector(m.collider.position);
        self.check_vector(m.old_position);
        assert_eq!(self.word(), u32::MAX);
        assert_eq!(m.collider.cell, usize::MAX);
        self.check_float(m.collider.radius);
        self.check_float(m.direction);
        self.check_float(m.look);
        self.check_float(m.last_camera_y);
        self.check_vector(m.camera_position);
        let hit = m.hit_cell.borrow();
        for matrix in [hit.matrix0, hit.matrix1] {
            for v in matrix {
                self.check_vector(v);
            }
        }
        assert_eq!(m.jump_timer, self.word() as i32);
        self.check_float(m.speed_up);
        assert_eq!(m.on_ground, self.word() != 0);
    }
}
#[test]
fn original_menu_world_player_placement() {
    let mut r = Reader(include_bytes!("fixtures/nullable-player.bin"));
    assert_eq!(r.word(), 64);
    let world = World {
        entries: vec![],
        adds: vec![],
        subs: vec![],
        zones: vec![],
        faces: vec![],
        connectivity: connectivity::connect(&[]),
    };
    for _ in 0..64 {
        let mut motion = Motion::spawn(&world, [0., 0., 0., 1.], 0.).unwrap();
        motion.matrix = kkrieger_oxide::mesh::identity();
        motion.jump_timer = 7;
        motion.speed_up = 0.2;
        motion.on_ground = true;
        let handle = Rc::clone(&motion.hit_cell);
        let position = r.vector();
        motion.place(&world, position, 0.25, -0.5).unwrap();
        r.placement(&motion);
        assert!(Rc::ptr_eq(&handle, &motion.hit_cell));
    }
    assert!(r.0.is_empty());
}
