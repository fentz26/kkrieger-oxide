use kkrieger_oxide::vfx::Bank;
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
    fn sample(&mut self) -> i16 {
        i16::from_le_bytes(self.take(2).try_into().unwrap())
    }
}
#[test]
fn original_vfx_plans_gain_fades_pcm_and_mono_downmix_match() {
    let mut r = Reader(include_bytes!("fixtures/vfx-pcm.bin"));
    let n = r.word() as usize;
    let bank = Bank::parse(r.take(n)).unwrap();
    assert_eq!(bank.sample_rate, 44100);
    assert_eq!(bank.len(), 32);
    for (case, entry) in bank.entries().enumerate() {
        let plan = entry.plan().unwrap();
        assert_eq!(plan.seek, r.word());
        let n = r.word() as usize;
        assert_eq!(plan.frames, n);
        let input: Vec<_> = (0..n).map(|_| [r.float(), r.float()]).collect();
        let mut pcm = vec![[0; 2]; n];
        plan.encode(&input, &mut pcm).unwrap();
        assert_eq!(n, r.word() as usize);
        assert_eq!(r.word(), 4);
        assert_eq!(u32::from(plan.slot), r.word());
        assert_eq!(u32::from(plan.spatial), r.word());
        for (sample, value) in pcm.iter().flatten().enumerate() {
            assert_eq!(*value, r.sample(), "case {case} PCM {sample}");
        }
        if plan.spatial {
            for (i, frame) in pcm.into_iter().enumerate() {
                assert_eq!(
                    ((i32::from(frame[0]) + i32::from(frame[1])) >> 1) as i16,
                    r.sample(),
                    "case {case} mono {i}"
                );
            }
        }
    }
    assert!(r.0.is_empty());
}
#[test]
fn authored_vfx_table_and_malformed_lengths_are_bounded() {
    let document =
        kkrieger_oxide::kx::Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let bank = Bank::parse(document.samples).unwrap();
    assert_eq!(bank.len(), 25);
    assert_eq!(bank.sample_rate, 44100);
    assert_eq!(bank.song.len(), 6224);
    assert_eq!(bank.entries().next().unwrap().plan().unwrap().frames, 10662);
    assert_eq!(bank.entries().last().unwrap().slot, 66);
    for n in [0, 4, 8, 12, 16, document.samples.len() - 1] {
        assert!(Bank::parse(&document.samples[..n]).is_err());
    }
    let mut bad = document.samples.to_vec();
    bad[..4].copy_from_slice(b"BAD0");
    assert!(Bank::parse(&bad).is_err());
    bad = document.samples.to_vec();
    bad[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(Bank::parse(&bad).is_err());
}

#[test]
fn all_authored_effects_render_to_their_original_slots_and_mono_sizes() {
    use kkrieger_oxide::vfx::Data;
    let document =
        kkrieger_oxide::kx::Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let bank = Bank::parse(document.samples).unwrap();
    let samples = bank.render(1234).unwrap();
    assert_eq!(samples.len(), 25);
    for (entry, sample) in bank.entries().zip(&samples) {
        let plan = entry.plan().unwrap();
        assert_eq!(sample.slot, entry.slot);
        assert_eq!(sample.buffers, 4);
        assert_eq!(sample.data.len(), plan.frames);
        let Data::Mono(pcm) = &sample.data else {
            panic!("authored 3D effect must register mono")
        };
        assert!(pcm.iter().all(|&v| v != -32768));
        if matches!(entry.slot, 0 | 1) {
            assert!(
                pcm.iter().any(|v| v.abs() > 1000),
                "effect {} lacks audible PCM",
                entry.slot
            );
        }
    }
}

#[test]
fn pcm_bounds_nonfinite_input_and_negative_spans_are_rejected() {
    use kkrieger_oxide::vfx::Entry;
    let entry = Entry {
        major: 0,
        slot: 0,
        gain: 0,
        reserved: 0,
        start: 10,
        end: 9,
        loop_size: 0,
    };
    assert!(entry.plan().is_err());
    let plan = Entry { end: 12, ..entry }.plan().unwrap();
    let mut pcm = [[123; 2]; 2];
    assert!(plan.encode(&[[0.; 2]], &mut pcm).is_err());
    assert_eq!(pcm, [[123; 2]; 2]);
    assert!(plan.encode(&[[f32::NAN; 2]; 2], &mut pcm).is_err());
    assert_eq!(pcm, [[123; 2]; 2]);
}
