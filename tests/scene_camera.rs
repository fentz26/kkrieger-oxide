use kkrieger_oxide::{
    kx::{Class, Document},
    scene::{event::Event, exec::Execution},
};
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let value = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        value
    }
    fn matrix(&mut self) -> [[f32; 4]; 4] {
        std::array::from_fn(|_| std::array::from_fn(|_| f32::from_bits(self.word())))
    }
}
#[test]
fn original_camera_transform_matches_native_and_does_not_execute_inputs() {
    let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    doc.events.clear();
    doc.classes = [0xc6, 0xffff]
        .into_iter()
        .map(|id| Class {
            id,
            convention: 0,
            packing: &[],
        })
        .collect();
    doc.operators.truncate(2);
    for (i, op) in doc.operators.iter_mut().enumerate() {
        op.animation = &[1];
        op.class = i;
        op.inputs.clear();
        op.links.clear();
    }
    doc.operators[0].inputs = vec![1];
    let mut r = Reader(include_bytes!("fixtures/scene-camera.bin"));
    assert_eq!(r.word(), 512);
    for case in 0..512 {
        doc.operators[0].parameters = (0..9).map(|_| r.word()).collect();
        let parent = r.matrix();
        let expected = r.matrix();
        assert_eq!(parent, r.matrix());
        assert_eq!(r.word(), 0);
        let mut e = Execution::new(&doc);
        e.game_camera.zoom_x = 1.7;
        let id = e.events.insert(Event::default(), false);
        e.execute_event(&doc, 0, id, parent).unwrap();
        for (a, b) in e
            .game_camera
            .space
            .into_iter()
            .flatten()
            .zip(expected.into_iter().flatten())
        {
            assert_eq!(a.to_bits(), b.to_bits(), "case {case}: {a} vs {b}");
        }
        assert_eq!(e.game_camera.zoom_x, 1.7);
        assert!(e.jobs.is_empty());
    }
    assert!(r.0.is_empty());
}
