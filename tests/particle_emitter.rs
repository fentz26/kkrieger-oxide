use kkrieger_oxide::{
    random::Random,
    scene::particles::{Emitter, Parameters, System},
};
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let n = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        n
    }
    fn float(&mut self) -> f32 {
        f32::from_bits(self.word())
    }
    fn vector(&mut self) -> [f32; 4] {
        std::array::from_fn(|_| self.float())
    }
    fn check(&mut self, v: [f32; 4]) {
        for n in v {
            assert_eq!(n.to_bits(), self.word());
        }
    }
}
#[test]
fn original_particle_emission_bursts_capacity_and_random_stream() {
    let mut r = Reader(include_bytes!("fixtures/particle-emitter.bin"));
    assert_eq!(r.word(), 32);
    assert_eq!(r.word(), 16);
    for _ in 0..32 {
        let capacity = r.word() as usize;
        let burst = r.word() as i32;
        let mode = r.word();
        let mut rng = Random::new(r.word() as i32);
        let p = Parameters {
            burst,
            mode,
            lifetime: r.float(),
            rate: r.float(),
            speed: r.vector(),
            random_position: r.vector(),
            random_speed: r.vector(),
        };
        let matrix = std::array::from_fn(|_| r.vector());
        let mut system = System {
            capacity,
            particles: vec![],
        };
        let mut emitter = Emitter::default();
        for _ in 0..16 {
            let time = r.word() as i32;
            let delta = r.word() as i32;
            emitter.emit(&mut system, &p, matrix, time, delta, &mut rng);
            assert_eq!(emitter.burst_count, r.word() as i32);
            assert_eq!(system.particles.len(), r.word() as usize);
            for particle in &system.particles {
                r.check(particle.position);
                r.check(particle.speed);
                assert_eq!(particle.time, r.word() as i32);
                assert_eq!(particle.max_time, r.word() as i32);
            }
        }
        assert_eq!(rng.next_u32(), r.word());
    }
    assert!(r.0.is_empty());
}
