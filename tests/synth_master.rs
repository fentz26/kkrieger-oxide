use kkrieger_oxide::synth::{master::Master, rate::Rate};
#[test]
fn globals_require_22_dsp_bytes_and_ignore_the_gui_byte() {
    let mut master = Master::default();
    master.init();
    let rate = Rate::new(44100).unwrap();
    assert!(master.set(&[0; 21], &rate).is_err());
    let mut globals = [0; 23];
    globals[12] = 127;
    globals[22] = 255;
    master.set(&globals, &rate).unwrap();
    assert_eq!(master.high_frequency.to_bits(), 1f32.to_bits());
    assert_eq!(master.low_frequency, 1. / 16384.);
}

struct Reader<'a>(&'a [u8]);
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> &'a [u8] {
        let (a, b) = self.0.split_at(n);
        self.0 = b;
        a
    }
    fn word(&mut self) -> u32 {
        u32::from_le_bytes(self.take(4).try_into().unwrap())
    }
    fn float(&mut self) -> f32 {
        f32::from_bits(self.word())
    }
}
fn snapshot(m: &Master) -> Vec<u32> {
    let mut state = Vec::new();
    state.extend([
        m.parameters[0].to_bits(),
        m.parameters[1].to_bits(),
        m.parameters[2].to_bits(),
        m.parameters[3].to_bits(),
        m.parameters[4].to_bits(),
        m.parameters[5].to_bits(),
        m.parameters[6].to_bits(),
        m.parameters[7].to_bits(),
        m.parameters[8].to_bits(),
        m.parameters[9].to_bits(),
        m.parameters[10].to_bits(),
        m.parameters[11].to_bits(),
        m.parameters[12].to_bits(),
        m.parameters[13].to_bits(),
        m.parameters[14].to_bits(),
        m.parameters[15].to_bits(),
        m.parameters[16].to_bits(),
        m.parameters[17].to_bits(),
        m.parameters[18].to_bits(),
        m.parameters[19].to_bits(),
        m.parameters[20].to_bits(),
        m.parameters[21].to_bits(),
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        m.reverb.gains[0].to_bits(),
        m.reverb.gains[1].to_bits(),
        m.reverb.gains[2].to_bits(),
        m.reverb.gains[3].to_bits(),
        m.reverb.gains[4].to_bits(),
        m.reverb.gains[5].to_bits(),
        m.reverb.input_gain.to_bits(),
        m.reverb.damping.to_bits(),
        m.reverb.low_cut.to_bits(),
        m.reverb.positions[0],
        m.reverb.positions[1],
        m.reverb.positions[2],
        m.reverb.positions[3],
        m.reverb.positions[4],
        m.reverb.positions[5],
        m.reverb.positions[6],
        m.reverb.positions[7],
        m.reverb.positions[8],
        m.reverb.positions[9],
        m.reverb.positions[10],
        m.reverb.positions[11],
        m.reverb.lowpass[0].to_bits(),
        m.reverb.lowpass[1].to_bits(),
        m.reverb.lowpass[2].to_bits(),
        m.reverb.lowpass[3].to_bits(),
        m.reverb.lowpass[4].to_bits(),
        m.reverb.lowpass[5].to_bits(),
        m.reverb.lowpass[6].to_bits(),
        m.reverb.lowpass[7].to_bits(),
        m.reverb.highpass[0].to_bits(),
        m.reverb.highpass[1].to_bits(),
    ]);
    state.extend(m.reverb.history.iter().map(|v| v.to_bits()));
    state.extend([
        0,
        0,
        m.delay.mask,
        m.delay.write_position,
        m.delay.left_offset,
        m.delay.right_offset,
        m.delay.counter,
        m.delay.frequency,
        m.delay.phase,
        m.delay.depth,
        m.delay.feedback.to_bits(),
        m.delay.dry.to_bits(),
        m.delay.effect.to_bits(),
        m.compressor.mode,
        m.compressor.old_mode,
        m.compressor.input_gain.to_bits(),
        m.compressor.ratio.to_bits(),
        m.compressor.output_gain.to_bits(),
        m.compressor.attack.to_bits(),
        m.compressor.release.to_bits(),
        m.compressor.lookahead_length,
        m.compressor.lookahead_position,
        m.compressor.current_gain[0].to_bits(),
        m.compressor.current_gain[1].to_bits(),
        m.compressor.peak[0].to_bits(),
        m.compressor.peak[1].to_bits(),
        m.compressor.rms_position,
        m.compressor.rms_value[0].to_bits(),
        m.compressor.rms_value[1].to_bits(),
    ]);
    state.extend(m.compressor.lookahead.iter().flatten().map(|v| v.to_bits()));
    state.extend(m.compressor.rms_history.iter().map(|v| v.to_bits()));
    state.extend([
        m.low_frequency.to_bits(),
        m.low_state[0].to_bits(),
        m.low_state[1].to_bits(),
        m.high_frequency.to_bits(),
        m.high_state[0].to_bits(),
        m.high_state[1].to_bits(),
    ]);
    state
}
#[test]
fn original_global_setup_and_128_complete_master_blocks_match() {
    let mut r = Reader(include_bytes!("fixtures/synth-master.bin"));
    assert_eq!(r.word(), 32);
    assert_eq!(r.word(), 4);
    let words = r.word() as usize;
    let mut levels = vec![[0.; 2]; 280];
    for case in 0..32 {
        let rate = Rate::new(r.word()).unwrap();
        let n = r.word() as usize;
        let globals = r.take(23);
        let mut master = Master::default();
        master.init();
        master.set(globals, &rate).unwrap();
        for block in 0..4 {
            let mut mix: Vec<_> = (0..n).map(|_| [r.float(), r.float()]).collect();
            let a: Vec<_> = (0..n).map(|_| r.float()).collect();
            let b: Vec<_> = (0..n).map(|_| r.float()).collect();
            master.process(&mut mix, &a, &b, &mut levels).unwrap();
            for (sample, value) in mix.into_iter().flatten().enumerate() {
                assert_eq!(
                    value.to_bits(),
                    r.word(),
                    "case {case} block {block} sample {sample}"
                );
            }
        }
        let mut expected = vec![0; words];
        let count = r.word();
        for _ in 0..count {
            let i = r.word() as usize;
            expected[i] = r.word();
        }
        let state = snapshot(&master);
        assert_eq!(state.len(), words);
        for (i, (value, expected)) in state.into_iter().zip(expected).enumerate() {
            assert_eq!(value, expected, "case {case} master state {i}");
        }
        for side in 0..2 {
            for (i, frame) in master.delay.history.iter().enumerate() {
                let expected = if i < n * 4 { r.word() } else { 0 };
                assert_eq!(
                    frame[side].to_bits(),
                    expected,
                    "case {case} delay {side} sample {i}"
                );
            }
        }
    }
    assert!(r.0.is_empty());
}

#[test]
fn short_master_buses_leave_audio_and_effect_state_unchanged() {
    let mut master = Master::default();
    master.init();
    let before = snapshot(&master);
    let mut mix = [[3., 4.]; 4];
    assert!(
        master
            .process(&mut mix, &[0.; 3], &[0.; 4], &mut [[0.; 2]; 4])
            .is_err()
    );
    assert_eq!(mix, [[3., 4.]; 4]);
    assert_eq!(snapshot(&master), before);
    assert!(
        master
            .process(&mut mix, &[0.; 4], &[0.; 4], &mut [[0.; 2]; 3])
            .is_err()
    );
    assert_eq!(mix, [[3., 4.]; 4]);
    assert_eq!(snapshot(&master), before);
    master.process(&mut [], &[], &[], &mut []).unwrap();
    assert_eq!(snapshot(&master), before);
}
