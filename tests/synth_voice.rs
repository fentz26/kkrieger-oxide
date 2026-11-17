use kkrieger_oxide::synth::{
    rate::Rate,
    voice::{Buffers, Parameters, Voice},
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
    fn state(&mut self, v: &Voice, case: usize, frame: usize) {
        for (field, value) in [
            v.note as u32,
            v.velocity.to_bits(),
            v.gate,
            v.current_volume.to_bits(),
            v.volume_ramp.to_bits(),
            v.transpose.to_bits(),
            v.routing,
            v.left_volume.to_bits(),
            v.right_volume.to_bits(),
            v.filter1_gain.to_bits(),
            v.filter2_gain.to_bits(),
            v.oscillator_sync,
            v.oscillators[0].mode,
            v.oscillators[0].ring,
            v.oscillators[0].counter,
            v.oscillators[0].frequency,
            v.oscillators[0].breakpoint,
            v.oscillators[0].noise_frequency.to_bits(),
            v.oscillators[0].noise_resonance.to_bits(),
            v.oscillators[0].seed,
            v.oscillators[0].gain.to_bits(),
            v.oscillators[0].oversampled_gain.to_bits(),
            v.oscillators[0].temporary,
            v.oscillators[0].noise_low.to_bits(),
            v.oscillators[0].noise_band.to_bits(),
            v.oscillators[0].note.to_bits(),
            v.oscillators[0].pitch.to_bits(),
            v.oscillators[1].mode,
            v.oscillators[1].ring,
            v.oscillators[1].counter,
            v.oscillators[1].frequency,
            v.oscillators[1].breakpoint,
            v.oscillators[1].noise_frequency.to_bits(),
            v.oscillators[1].noise_resonance.to_bits(),
            v.oscillators[1].seed,
            v.oscillators[1].gain.to_bits(),
            v.oscillators[1].oversampled_gain.to_bits(),
            v.oscillators[1].temporary,
            v.oscillators[1].noise_low.to_bits(),
            v.oscillators[1].noise_band.to_bits(),
            v.oscillators[1].note.to_bits(),
            v.oscillators[1].pitch.to_bits(),
            v.oscillators[2].mode,
            v.oscillators[2].ring,
            v.oscillators[2].counter,
            v.oscillators[2].frequency,
            v.oscillators[2].breakpoint,
            v.oscillators[2].noise_frequency.to_bits(),
            v.oscillators[2].noise_resonance.to_bits(),
            v.oscillators[2].seed,
            v.oscillators[2].gain.to_bits(),
            v.oscillators[2].oversampled_gain.to_bits(),
            v.oscillators[2].temporary,
            v.oscillators[2].noise_low.to_bits(),
            v.oscillators[2].noise_band.to_bits(),
            v.oscillators[2].note.to_bits(),
            v.oscillators[2].pitch.to_bits(),
            v.filters[0].mode,
            v.filters[0].frequency.to_bits(),
            v.filters[0].resonance.to_bits(),
            v.filters[0].low.to_bits(),
            v.filters[0].band.to_bits(),
            v.filters[0].step_bytes,
            v.filters[1].mode,
            v.filters[1].frequency.to_bits(),
            v.filters[1].resonance.to_bits(),
            v.filters[1].low.to_bits(),
            v.filters[1].band.to_bits(),
            v.filters[1].step_bytes,
            v.envelopes[0].output.to_bits(),
            v.envelopes[0].state,
            v.envelopes[0].value.to_bits(),
            v.envelopes[0].attack_delta.to_bits(),
            v.envelopes[0].decay_factor.to_bits(),
            v.envelopes[0].sustain_level.to_bits(),
            v.envelopes[0].sustain_factor.to_bits(),
            v.envelopes[0].release_factor.to_bits(),
            v.envelopes[0].gain.to_bits(),
            v.envelopes[1].output.to_bits(),
            v.envelopes[1].state,
            v.envelopes[1].value.to_bits(),
            v.envelopes[1].attack_delta.to_bits(),
            v.envelopes[1].decay_factor.to_bits(),
            v.envelopes[1].sustain_level.to_bits(),
            v.envelopes[1].sustain_factor.to_bits(),
            v.envelopes[1].release_factor.to_bits(),
            v.envelopes[1].gain.to_bits(),
            v.lfos[0].output.to_bits(),
            v.lfos[0].mode,
            v.lfos[0].sync,
            v.lfos[0].envelope,
            v.lfos[0].frequency,
            v.lfos[0].counter,
            v.lfos[0].phase,
            v.lfos[0].gain.to_bits(),
            v.lfos[0].dc.to_bits(),
            v.lfos[0].seed,
            v.lfos[0].last,
            v.lfos[1].output.to_bits(),
            v.lfos[1].mode,
            v.lfos[1].sync,
            v.lfos[1].envelope,
            v.lfos[1].frequency,
            v.lfos[1].counter,
            v.lfos[1].phase,
            v.lfos[1].gain.to_bits(),
            v.lfos[1].dc.to_bits(),
            v.lfos[1].seed,
            v.lfos[1].last,
            v.distortion.mode,
            v.distortion.input_gain.to_bits(),
            v.distortion.output_gain.to_bits(),
            v.distortion.offset.to_bits(),
            v.distortion.crush_scale.to_bits(),
            v.distortion.crush_step as u32,
            v.distortion.crush_xor,
            v.distortion.counter,
            v.distortion.frequency,
            v.distortion.held_left.to_bits(),
            v.distortion.held_right.to_bits(),
            v.distortion.pre_coefficient.to_bits(),
            v.distortion.pre_left.to_bits(),
            v.distortion.pre_right.to_bits(),
            v.distortion.post_coefficient.to_bits(),
            v.distortion.post_left.to_bits(),
            v.distortion.post_right.to_bits(),
            v.distortion.left_filter.mode,
            v.distortion.left_filter.frequency.to_bits(),
            v.distortion.left_filter.resonance.to_bits(),
            v.distortion.left_filter.low.to_bits(),
            v.distortion.left_filter.band.to_bits(),
            v.distortion.left_filter.step_bytes,
            v.distortion.right_filter.mode,
            v.distortion.right_filter.frequency.to_bits(),
            v.distortion.right_filter.resonance.to_bits(),
            v.distortion.right_filter.low.to_bits(),
            v.distortion.right_filter.band.to_bits(),
            v.distortion.right_filter.step_bytes,
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(
                value,
                self.word(),
                "case {case} frame {frame} field {field}"
            );
        }
    }
    fn stereo(&mut self, n: usize) -> Vec<[f32; 2]> {
        (0..n).map(|_| [self.float(), self.float()]).collect()
    }
}
#[test]
fn complete_original_x87_voices_match_state_and_stereo_samples() {
    let mut r = Reader(include_bytes!("fixtures/synth-voice.bin"));
    assert_eq!(r.word(), 64);
    assert_eq!(r.word(), 6);
    let rate = Rate::new(44100).unwrap();
    let mut buffers = Buffers::default();
    for case in 0..64 {
        let values: [f32; 59] = std::array::from_fn(|_| r.float());
        let seeds = std::array::from_fn(|_| r.word());
        let mut voice = Voice::default();
        voice.init(seeds);
        voice
            .set(Parameters::from_values(&values).unwrap(), &rate)
            .unwrap();
        r.state(&voice, case, 99);
        for frame in 0..6 {
            if frame == 0 || frame == 3 {
                voice.note_on(
                    48 + case as i32 % 24 + if frame == 3 { 2 } else { 0 },
                    80 + case as i32 % 48,
                    &rate,
                );
            } else if frame == 2 || frame == 5 {
                voice.note_off();
            }
            voice.tick().unwrap();
            let n = r.word() as usize;
            let mut channel = r.stereo(n);
            let a = r.stereo(n);
            let b = r.stereo(n);
            voice.render(&mut channel, &a, &b, &mut buffers).unwrap();
            r.state(&voice, case, frame);
            for (sample, output) in channel.into_iter().flatten().enumerate() {
                assert_eq!(
                    output.to_bits(),
                    r.word(),
                    "case {case} frame {frame} sample {sample}"
                );
            }
        }
    }
    assert!(r.0.is_empty());
}

#[test]
fn voice_rejects_oversized_or_missing_auxiliary_buffers_before_rendering() {
    let mut voice = Voice::default();
    let mut buffers = Buffers::default();
    let mut channel = vec![[3., 4.]; 281];
    assert!(voice.render(&mut channel, &[], &[], &mut buffers).is_err());
    assert!(channel.iter().all(|v| *v == [3., 4.]));
    channel.truncate(128);
    voice.oscillators[2].mode = 6;
    assert!(voice.render(&mut channel, &[], &[], &mut buffers).is_err());
    assert_eq!(voice.oscillators[0].counter, 0);
    assert!(channel.iter().all(|v| *v == [3., 4.]));
    voice.render(&mut [], &[], &[], &mut buffers).unwrap();
}

#[test]
fn retrigger_retains_envelope_level_and_volume_ramp() {
    let mut voice = Voice::default();
    voice.envelopes[0].value = 31.;
    voice.envelopes[1].value = 47.;
    voice.current_volume = 0.25;
    voice.volume_ramp = 0.125;
    voice.oscillators[0].counter = 123;
    let rate = Rate::new(48000).unwrap();
    voice.note_on(64, 100, &rate);
    assert_eq!(voice.envelopes[0].value, 31.);
    assert_eq!(voice.envelopes[1].value, 47.);
    assert_eq!(voice.current_volume, 0.25);
    assert_eq!(voice.volume_ramp, 0.125);
    assert_eq!(voice.oscillators[0].counter, 123);
    voice.oscillator_sync = 1;
    voice.note_on(65, 80, &rate);
    assert!(voice.oscillators.iter().all(|o| o.counter == 0));
    voice.note_off();
    assert_eq!(voice.gate, 0);
    assert_eq!(voice.envelopes[0].state, 1);
}
