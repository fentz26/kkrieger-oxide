use kkrieger_oxide::synth::{
    channel::{Channel, Parameters},
    rate::Rate,
};
#[test]
fn channel_parameter_layout_matches_original_29_values() {
    let values = std::array::from_fn::<_, 29, _>(|i| i as f32);
    let p = Parameters::from_values(&values).unwrap();
    let mut channel = Channel::default();
    channel.init();
    channel.set(p, &Rate::new(44100).unwrap()).unwrap();
    assert_eq!(channel.routing, 7);
    assert_eq!(channel.receive_a, 1. / 128.);
    assert!(Parameters::from_values(&values[..28]).is_err());
}

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
    fn stereo(&mut self, n: usize) -> Vec<[f32; 2]> {
        (0..n).map(|_| [self.float(), self.float()]).collect()
    }
    fn mono(&mut self, n: usize) -> Vec<f32> {
        (0..n).map(|_| self.float()).collect()
    }
    fn check(
        &mut self,
        values: impl IntoIterator<Item = f32>,
        case: usize,
        block: usize,
        bus: usize,
    ) {
        for (sample, value) in values.into_iter().enumerate() {
            assert_eq!(
                value.to_bits(),
                self.word(),
                "case {case} block {block} bus {bus} sample {sample}"
            );
        }
    }
}
fn snapshot(c: &Channel) -> Vec<u32> {
    let mut state = vec![
        c.gain.to_bits(),
        c.auxiliary1_gain.to_bits(),
        c.auxiliary2_gain.to_bits(),
        c.send_a.to_bits(),
        c.send_b.to_bits(),
        c.receive_a.to_bits(),
        c.receive_b.to_bits(),
        c.routing,
        c.boost.enabled,
        c.boost.a1.to_bits(),
        c.boost.a2.to_bits(),
        c.boost.b0.to_bits(),
        c.boost.b1.to_bits(),
        c.boost.b2.to_bits(),
        c.boost.x1[0].to_bits(),
        c.boost.x1[1].to_bits(),
        c.boost.x2[0].to_bits(),
        c.boost.x2[1].to_bits(),
        c.boost.y1[0].to_bits(),
        c.boost.y1[1].to_bits(),
        c.boost.y2[0].to_bits(),
        c.boost.y2[1].to_bits(),
        c.distortion.mode,
        c.distortion.input_gain.to_bits(),
        c.distortion.output_gain.to_bits(),
        c.distortion.offset.to_bits(),
        c.distortion.crush_scale.to_bits(),
        c.distortion.crush_step as u32,
        c.distortion.crush_xor,
        c.distortion.counter,
        c.distortion.frequency,
        c.distortion.held_left.to_bits(),
        c.distortion.held_right.to_bits(),
        c.distortion.pre_coefficient.to_bits(),
        c.distortion.pre_left.to_bits(),
        c.distortion.pre_right.to_bits(),
        c.distortion.post_coefficient.to_bits(),
        c.distortion.post_left.to_bits(),
        c.distortion.post_right.to_bits(),
        c.distortion.left_filter.mode,
        c.distortion.left_filter.frequency.to_bits(),
        c.distortion.left_filter.resonance.to_bits(),
        c.distortion.left_filter.low.to_bits(),
        c.distortion.left_filter.band.to_bits(),
        c.distortion.left_filter.step_bytes,
        c.distortion.right_filter.mode,
        c.distortion.right_filter.frequency.to_bits(),
        c.distortion.right_filter.resonance.to_bits(),
        c.distortion.right_filter.low.to_bits(),
        c.distortion.right_filter.band.to_bits(),
        c.distortion.right_filter.step_bytes,
        0,
        0,
        c.delay.mask,
        c.delay.write_position,
        c.delay.left_offset,
        c.delay.right_offset,
        c.delay.counter,
        c.delay.frequency,
        c.delay.phase,
        c.delay.depth,
        c.delay.feedback.to_bits(),
        c.delay.dry.to_bits(),
        c.delay.effect.to_bits(),
        c.compressor.mode,
        c.compressor.old_mode,
        c.compressor.input_gain.to_bits(),
        c.compressor.ratio.to_bits(),
        c.compressor.output_gain.to_bits(),
        c.compressor.attack.to_bits(),
        c.compressor.release.to_bits(),
        c.compressor.lookahead_length,
        c.compressor.lookahead_position,
        c.compressor.current_gain[0].to_bits(),
        c.compressor.current_gain[1].to_bits(),
        c.compressor.peak[0].to_bits(),
        c.compressor.peak[1].to_bits(),
        c.compressor.rms_position,
        c.compressor.rms_value[0].to_bits(),
        c.compressor.rms_value[1].to_bits(),
    ];
    state.extend(c.compressor.lookahead.iter().flatten().map(|v| v.to_bits()));
    state.extend(c.compressor.rms_history.iter().map(|v| v.to_bits()));
    state
}
#[test]
fn complete_original_channel_effect_orders_receives_and_all_sends_match() {
    let mut r = Reader(include_bytes!("fixtures/synth-channel.bin"));
    assert_eq!(r.word(), 32);
    assert_eq!(r.word(), 4);
    let words = r.word() as usize;
    let mut levels = vec![[0.; 2]; 280];
    for case in 0..32 {
        let n = r.word() as usize;
        let rate = Rate::new(r.word()).unwrap();
        let values: [f32; 29] = std::array::from_fn(|_| r.float());
        let mut c = Channel::default();
        c.init();
        c.set(Parameters::from_values(&values).unwrap(), &rate)
            .unwrap();
        for block in 0..4 {
            let mut audio = r.stereo(n);
            let mut mix = r.stereo(n);
            let mut a1 = r.mono(n);
            let mut a2 = r.mono(n);
            let mut aa = r.stereo(n);
            let mut ab = r.stereo(n);
            c.process(
                &mut audio,
                &mut kkrieger_oxide::synth::channel::Buses {
                    mix: &mut mix,
                    auxiliary1: &mut a1,
                    auxiliary2: &mut a2,
                    auxiliary_a: &mut aa,
                    auxiliary_b: &mut ab,
                },
                &mut levels,
            )
            .unwrap();
            r.check(audio.into_iter().flatten(), case, block, 0);
            r.check(mix.into_iter().flatten(), case, block, 1);
            r.check(a1, case, block, 2);
            r.check(a2, case, block, 3);
            r.check(aa.into_iter().flatten(), case, block, 4);
            r.check(ab.into_iter().flatten(), case, block, 5);
        }
        let mut expected = vec![0; words];
        let count = r.word();
        for _ in 0..count {
            let index = r.word() as usize;
            expected[index] = r.word();
        }
        let state = snapshot(&c);
        assert_eq!(state.len(), words);
        for (i, (value, expected)) in state.into_iter().zip(expected).enumerate() {
            assert_eq!(value, expected, "case {case} state {i}");
        }
        for side in 0..2 {
            for (i, frame) in c.delay.history.iter().enumerate() {
                assert_eq!(
                    frame[side].to_bits(),
                    r.word(),
                    "case {case} delay {side} sample {i}"
                );
            }
        }
    }
    assert!(r.0.is_empty());
}

#[test]
fn short_buses_are_rejected_before_audio_or_effect_state_changes() {
    let mut c = Channel::default();
    c.init();
    let before = snapshot(&c);
    let mut audio = [[3., 4.]; 4];
    let mut mix = [[0.; 2]; 4];
    let mut a1 = [0.; 3];
    let mut a2 = [0.; 4];
    let mut aa = [[0.; 2]; 4];
    let mut ab = [[0.; 2]; 4];
    let mut buses = kkrieger_oxide::synth::channel::Buses {
        mix: &mut mix,
        auxiliary1: &mut a1,
        auxiliary2: &mut a2,
        auxiliary_a: &mut aa,
        auxiliary_b: &mut ab,
    };
    assert!(
        c.process(&mut audio, &mut buses, &mut [[0.; 2]; 4])
            .is_err()
    );
    assert_eq!(audio, [[3., 4.]; 4]);
    assert_eq!(snapshot(&c), before);
    c.process(&mut [], &mut buses, &mut []).unwrap();
    assert_eq!(snapshot(&c), before);
}
