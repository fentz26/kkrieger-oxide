use kkrieger_oxide::scene::{camera::Environment, viewport::Parameters};
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
    fn read_vector(&mut self) -> [f32; 4] {
        std::array::from_fn(|_| self.float())
    }
    fn read_matrix(&mut self) -> [[f32; 4]; 4] {
        std::array::from_fn(|_| self.read_vector())
    }
    fn read_camera(&mut self) -> Environment {
        Environment {
            light_position: self.read_vector(),
            light_color: self.read_vector(),
            light_range: self.float(),
            light_amplify: self.float(),
            fog_start: self.float(),
            fog_end: self.float(),
            fog_color: self.word(),
            model: self.read_matrix(),
            space: self.read_matrix(),
            zoom_x: self.float(),
            zoom_y: self.float(),
            center_x: self.float(),
            center_y: self.float(),
            near_clip: self.float(),
            far_clip: self.float(),
            orthogonal: self.int(),
            released_projection: false,
        }
    }
    fn int(&mut self) -> i32 {
        self.word() as i32
    }
    fn check_float(&mut self, v: f32) {
        let expected = self.word();
        if v.is_nan() {
            assert!(f32::from_bits(expected).is_nan());
        } else {
            assert_eq!(v.to_bits(), expected, "{v}");
        }
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
fn original_viewport_camera_modes_projection_fog_and_spline_override() {
    let mut r = Reader(include_bytes!("fixtures/viewport-camera.bin"));
    assert_eq!(r.word(), 1024);
    for case in 0..1024 {
        let words: [u32; 18] = std::array::from_fn(|_| r.word());
        let params = Parameters::from_words(&words).unwrap();
        let game = r.read_camera();
        let aspect = r.float();
        let spline = if r.word() != 0 {
            Some((r.read_matrix(), r.float()))
        } else {
            None
        };
        let eye_distance = r.float();
        let focal = r.float();
        let stereo = r.word() != 0;
        let camera = params.camera(&game, aspect, spline);
        r.camera(&camera);
        let passes = params.passes(&camera, stereo, eye_distance, focal);
        assert_eq!(passes.cameras.len(), r.word() as usize);
        assert_eq!(passes.flags, r.word());
        assert_eq!(u32::from(passes.allocate_stereo_target), r.word());
        for camera in passes.cameras {
            r.camera(&camera);
        }
        assert_eq!(
            game.space[3][3], 2.,
            "input game camera changed in case {case}"
        );
    }
    assert!(r.0.is_empty());
    assert!(Parameters::from_words(&[0; 17]).is_err());
}
