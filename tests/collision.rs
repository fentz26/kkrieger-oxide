use kkrieger_oxide::collision::Cell;
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let v = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        v
    }
    fn vector(&mut self) -> [f32; 4] {
        std::array::from_fn(|_| f32::from_bits(self.word()))
    }
    fn matrix(&mut self) -> kkrieger_oxide::mesh::Matrix {
        std::array::from_fn(|_| self.vector())
    }
}
#[test]
fn original_collision_cells_include_warped_and_degenerate_faces() {
    let mut r = Reader(include_bytes!("fixtures/collision.bin"));
    assert_eq!(r.word(), 256);
    for case in 0..256 {
        let mode = r.word();
        let vertices = std::array::from_fn(|_| r.vector());
        let cell = Cell::from_vertices(vertices, mode);
        assert_eq!(cell.mode, r.word());
        assert_eq!(u32::from(cell.only_for_shot), r.word());
        assert_eq!(cell.planes.len(), r.word() as usize, "case {case}");
        for actual in
            cell.planes
                .iter()
                .copied()
                .chain([cell.center, cell.bounds[0], cell.bounds[1]])
        {
            let expected = r.vector();
            for (a, b) in actual.into_iter().zip(expected) {
                assert_eq!(a.to_bits(), b.to_bits(), "case {case}: {a} vs {b}");
            }
        }
    }
    assert!(r.0.is_empty());
}

#[test]
fn original_dynamic_cells_prepare_move_and_interpolate() {
    use kkrieger_oxide::collision::DynamicCell;
    fn check(r: &mut Reader<'_>, cell: &DynamicCell, case: usize, phase: usize) {
        assert_eq!(cell.cell.mode, r.word());
        assert_eq!(cell.blocked, r.word() as i32);
        assert_eq!(u32::from(cell.monster.is_some()), r.word());
        let values = cell
            .cell
            .planes
            .iter()
            .copied()
            .chain(cell.cell.vertices)
            .chain([cell.cell.center, cell.cell.bounds[0], cell.cell.bounds[1]])
            .chain(
                [
                    cell.confirmed_matrix,
                    cell.current_matrix,
                    cell.matrix0,
                    cell.matrix1,
                ]
                .into_iter()
                .flatten(),
            );
        for actual in values {
            let expected = r.vector();
            for (a, b) in actual.into_iter().zip(expected) {
                assert_eq!(
                    a.to_bits(),
                    b.to_bits(),
                    "case {case} phase {phase}: {a} vs {b}"
                );
            }
        }
    }
    let mut r = Reader(include_bytes!("fixtures/collision-dynamic.bin"));
    assert_eq!(r.word(), 128);
    for case in 0..128 {
        let vertices = std::array::from_fn(|_| r.vector());
        let a = r.matrix();
        let b = r.matrix();
        let fade = f32::from_bits(r.word());
        let mut cell = DynamicCell::new(Cell::from_vertices(vertices, 1), a);
        check(&mut r, &cell, case, 0);
        cell.blocked = 1;
        cell.apply_matrix(b);
        check(&mut r, &cell, case, 1);
        cell.matrix1 = b;
        cell.physics(fade);
        check(&mut r, &cell, case, 2);
    }
    assert!(r.0.is_empty());
}

#[test]
fn original_cell_queries_include_parallel_rays_and_degenerate_cells() {
    use kkrieger_oxide::collision::Intersection;
    fn check(r: &mut Reader<'_>, hit: Option<Intersection>, case: usize, query: usize) {
        assert_eq!(hit.is_some(), r.word() != 0, "{case}/{query}");
        if let Some(hit) = hit {
            for (a, b) in hit.plane.into_iter().zip(r.vector()) {
                assert_eq!(a.to_bits(), b.to_bits(), "{case}/{query}");
            }
            assert_eq!(hit.distance.to_bits(), r.word(), "{case}/{query}");
        }
    }
    let mut r = Reader(include_bytes!("fixtures/collision-queries.bin"));
    assert_eq!(r.word(), 256);
    assert_eq!(r.word(), 16);
    for case in 0..256 {
        let mode = r.word();
        let vertices = std::array::from_fn(|_| r.vector());
        let cell = Cell::from_vertices(vertices, mode);
        for query in 0..16 {
            let point = r.vector();
            let direction = r.vector();
            let other = r.vector();
            let radius = f32::from_bits(r.word());
            assert_eq!(cell.outside_mask(point, radius), r.word(), "{case}/{query}");
            assert_eq!(
                cell.reject_plane(point, other),
                r.word() != 0,
                "{case}/{query}"
            );
            assert_eq!(
                cell.approximate_distance(point).to_bits(),
                r.word(),
                "{case}/{query}"
            );
            check(
                &mut r,
                cell.intersect_out(point, direction, radius),
                case,
                query,
            );
            check(
                &mut r,
                cell.intersect_in(point, direction, radius),
                case,
                query,
            );
        }
    }
    assert!(r.0.is_empty());
}
