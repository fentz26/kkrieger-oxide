use kkrieger_oxide::{
    game::frame::Clock,
    kx::{Class, Document},
    scene::{camera::Environment, event::Event, exec::Execution},
};
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let v = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        v
    }
    fn int(&mut self) -> i32 {
        self.word() as i32
    }
    fn check_float(&mut self, v: f32) {
        assert_eq!(v.to_bits(), self.word(), "{v}");
    }
    fn vector(&mut self, v: [f32; 4]) {
        for x in v {
            self.check_float(x);
        }
    }
    fn camera(&mut self, c: &Environment) {
        self.vector(c.light_position);
        self.vector(c.light_color);
        for v in [c.light_range, c.light_amplify, c.fog_start, c.fog_end] {
            self.check_float(v);
        }
        assert_eq!(c.fog_color, self.word());
        for m in [c.model, c.space] {
            for v in m {
                self.vector(v);
            }
        }
        for v in [
            c.zoom_x,
            c.zoom_y,
            c.center_x,
            c.center_y,
            c.near_clip,
            c.far_clip,
        ] {
            self.check_float(v);
        }
        assert_eq!(c.orthogonal, self.int());
    }
}
#[test]
fn original_frame_initialization_cameras_and_main_timing_match_native() {
    let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    doc.events.clear();
    doc.operators[0].animation = &[2, 0, 0x8f, 5, 1];
    let mut r = Reader(include_bytes!("fixtures/frame.bin"));
    assert_eq!(r.word(), 64);
    assert_eq!(r.word(), 12);
    for case in 0..64 {
        let mut execution = Execution::new(&doc);
        execution.current_time = r.int();
        let default_present = r.word() != 0;
        for _ in 0..case % 4 {
            execution.events.insert(
                Event {
                    operator: Some(0),
                    ..Event::default()
                },
                true,
            );
        }
        for _ in 0..12 {
            let time = r.int();
            let beat = r.int();
            execution.vm.variables.fill([9.; 4]);
            let mut parameters = doc.operators[0].parameters.clone();
            let saved = execution
                .vm
                .execute(&doc.operators[0], &mut parameters, &doc.splines, None)
                .unwrap();
            assert_eq!(saved.saved_variables, 1);
            execution.event_monster = Some(9);
            execution.event_spline = Some(8);
            execution.current_event = Some(7);
            execution.game_camera.near_clip = 1.;
            execution.current_camera.far_clip = 2.;
            execution.init_frame(beat, time, false);
            for v in execution.vm.variables {
                r.vector(v);
            }
            for v in [
                execution.events.beat,
                execution.last_time,
                execution.current_time,
                execution.time_delta,
                i32::from(execution.time_reset),
                execution.time_slices,
                execution.time_jitter,
            ] {
                assert_eq!(v, r.int());
            }
            assert_eq!(r.word(), 0);
            assert!(execution.vm.restore(1).is_err());
            assert_eq!(r.word(), 0);
            assert_eq!(execution.event_spline.is_none(), r.word() != 0);
            assert_eq!(execution.event_monster.is_none(), r.word() != 0);
            assert_eq!(execution.current_event.is_none(), r.word() != 0);
            assert_eq!(r.word(), 1);
            assert_eq!(r.word() != 0, default_present);
            assert!(execution.next_monster_override.is_none());
            for _ in 0..4 {
                assert_eq!(r.word(), 0);
            }
            assert_eq!(execution.events.active(0).len(), r.word() as usize);
            r.camera(&execution.game_camera);
            r.camera(&execution.current_camera);
            execution.finish_events();
        }
    }
    assert_eq!(r.word(), 32);
    assert_eq!(r.word(), 8);
    for _ in 0..32 {
        let mut clock = Clock::new(r.int());
        let bpm = r.word();
        for _ in 0..8 {
            let frame = clock.advance(r.int(), bpm);
            for v in [frame.time, frame.beat, frame.slices] {
                assert_eq!(v, r.int());
            }
            r.check_float(frame.milliseconds as f32);
            r.check_float(clock.smoothed_ms);
            r.check_float(frame.fps);
            assert_eq!(clock.last_time, r.int());
        }
    }
    assert!(r.0.is_empty());
}
#[test]
fn clear_instance_flag_resets_registered_static_event_memory() {
    let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    doc.events.clear();
    doc.classes = vec![
        Class {
            id: 6,
            convention: 0,
            packing: &[],
        },
        Class {
            id: 0x14,
            convention: 0,
            packing: &[],
        },
    ];
    doc.operators.truncate(2);
    for op in &mut doc.operators {
        op.animation = &[1];
        op.inputs.clear();
        op.links.clear();
    }
    doc.operators[0].class = 0;
    doc.operators[0].parameters = vec![1f32.to_bits()];
    doc.operators[0].inputs = vec![1];
    doc.operators[1].class = 1;
    doc.operators[1].parameters = vec![10, 8f32.to_bits(), 1f32.to_bits()];
    let mut execution = Execution::new(&doc);
    let id = execution.events.insert(
        Event {
            operator: Some(0),
            ..Event::default()
        },
        false,
    );
    for (clear, expected) in [(false, true), (false, false), (true, true)] {
        execution.init_frame(0, 0, clear);
        assert!(execution.add_event(id));
        execution.splash.enabled = false;
        execution.frame(&doc, 0).unwrap();
        assert_eq!(execution.splash.enabled, expected);
        execution.finish_events();
        assert!(!execution.clear_instances);
    }
}

#[test]
fn compiled_player_letterbox_uses_integer_edges() {
    use kkrieger_oxide::game::frame::master_window;
    assert_eq!(master_window(1280, 720), [0, 120, 1280, 600]);
    assert_eq!(master_window(641, 481), [0, 80, 641, 400]);
}
