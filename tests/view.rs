use kkrieger_oxide::{
    kx::Document,
    mesh::identity,
    scene::{event::Event, exec::Execution},
};
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let n = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        n
    }
}
#[test]
fn original_view_reset_values_and_dynamic_event_removal() {
    let doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let mut r = Reader(include_bytes!("fixtures/view.bin"));
    assert_eq!(r.word(), 64);
    for case in 0..64 {
        let mut e = Execution::new(&doc);
        let document = e.events.document.clone();
        let persistent = e.events.insert(
            Event {
                operator: Some(0),
                ..Event::default()
            },
            false,
        );
        let handle = e.events.events[&persistent].clone();
        handle.borrow_mut().modulation = 0.75;
        let mut dynamic = vec![];
        for _ in 0..case % 5 {
            dynamic.push(e.events.insert(Event::default(), true));
        }
        e.current_time = case + 99;
        e.last_time = case + 99;
        e.time_delta = case + 99;
        e.time_slices = case + 99;
        e.time_jitter = case + 99;
        e.events.beat = case + 99;
        e.time_reset = false;
        e.aspect = case as f32 + 2.;
        e.markers.fill([[17.; 4]; 4]);
        e.reset_view();
        assert_eq!(e.events.dynamic.len(), r.word() as usize);
        assert_eq!(u32::from(e.clear_instances), r.word());
        // The native instance pointers are null; owned scope cleanup and cursor
        // reuse are exercised with actual nested actors in monster_scene.rs.
        for _ in 0..3 {
            assert_eq!(r.word(), 1);
        }
        assert_eq!(e.aspect.to_bits(), r.word());
        for n in [
            e.events.beat,
            e.current_time,
            e.last_time,
            e.time_delta,
            e.time_slices,
            e.time_jitter,
            i32::from(e.time_reset),
        ] {
            assert_eq!(n, r.word() as i32);
        }
        for matrix in e.markers {
            for vector in matrix {
                for n in vector {
                    assert_eq!(n.to_bits(), r.word());
                }
            }
        }
        assert_eq!(e.game.monsters.len(), r.word() as usize);
        assert_eq!(r.word(), (case % 5 + 1) as u32); // Native chains deleted.
        assert_eq!(e.markers, [identity(); 32]);
        assert!(dynamic.iter().all(|id| !e.events.events.contains_key(id)));
        assert_eq!(e.events.document, document);
        assert!(document.iter().all(|id| e.events.events.contains_key(id)));
        assert!(std::rc::Rc::ptr_eq(&handle, &e.events.events[&persistent]));
        assert_eq!(handle.borrow().modulation, 0.75);
    }
    assert!(r.0.is_empty());
}
