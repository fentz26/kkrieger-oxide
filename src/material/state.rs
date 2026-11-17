//! Material11's original 48-word CPU state, before shader compilation.
use super::Usage;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    pub words: [u32; 48],
    pub textures: [Option<usize>; 4],
}
impl Default for State {
    fn default() -> Self {
        let mut words = [0; 48];
        words[0] = 0x300;
        words[4..8].fill(2);
        words[28..32].fill(u32::MAX);
        words[32] = 8.0f32.to_bits();
        Self {
            words,
            textures: [None; 4],
        }
    }
}
impl State {
    /// Shader generation and fixed pipeline state consume words 0..24 and
    /// texture identities. Words 24..48 feed registers at Set/draw time only.
    pub fn same_gpu_program(&self, other: &Self) -> bool {
        self.words[..24] == other.words[..24] && self.textures == other.textures
    }
    pub(super) fn build(p: &[u32; 64], textures: [Option<usize>; 8], phase: Usage) -> Self {
        let flags = p[48];
        let mut state = Self::default();
        let w = &mut state.words;
        if flags & 0x1e == 0 {
            w.copy_from_slice(&p[..48]);
            state.textures.copy_from_slice(&textures[..4]);
            if flags & 0x30000 != 0 {
                w[0] |= 0x20000;
            }
            return state;
        }
        match phase {
            Usage::Base => {
                w[0] = 0x20300 | (p[0] & 2);
                w[28] = if flags & 2 != 0 { p[53] } else { 0x00ffffff };
                w[9] = 0x10;
                w[19] = 0x20;
                w[23] = 13;
                if flags & 0x400 != 0 {
                    state.textures[0] = textures[0];
                    w[4] = p[4];
                    w[24] = p[24];
                    w[23] = 4;
                    w[0] |= 1;
                }
            }
            Usage::Light => {
                w[0] = 0x300280 | (p[0] & 2) | if flags & 0x4000 != 0 { 0x5000 } else { 0x2000 };
                w[2] = 5;
                w[32] = p[51];
                w[28] = p[52];
                w[8] = 0x10;
                w[9] = 0x40;
                w[23] = 13;
                w[34..48].copy_from_slice(&p[34..48]);
                if textures[4].is_some() {
                    state.textures[1] = textures[4];
                    w[5] = p[56];
                    w[25] = p[60];
                }
                if flags & 0x200 == 0 {
                    w[1] |= 0x40;
                } else if flags & 0x2000 != 0 {
                    w[1] |= 0x80;
                }
            }
            Usage::Shadow => {
                w[0] = 0x70220;
                w[9] = 0x10;
            }
            Usage::PostLight => {
                w.copy_from_slice(&p[..48]);
                w[0] = 0x4280;
                w[23] = 12;
                state.textures.copy_from_slice(&textures[..4]);
            }
            Usage::PostLight2 => {
                w[0] = 0x280 | if flags & 0x8000 != 0 { 0 } else { 0x2000 };
                w[1] = if flags & 0x100 != 0 { 0x10 } else { 0x20 };
                state.textures[2] = textures[6];
                w[30] = p[53];
                w[6] = p[58];
                w[26] = p[62];
                w[17] = 0x10;
                w[34..48].copy_from_slice(&p[34..48]);
                if flags & 0x4000 != 0 {
                    state.textures[0] = textures[0];
                    w[4] = p[4];
                    w[24] = p[24];
                    w[17] |= 4;
                }
            }
            _ => {}
        }
        state
    }

    /// Exec_Material_Material updates every inherited pass using the current
    /// operator's animated parameters, exactly as the source does.
    pub(super) fn execute(&mut self, p: &[u32; 64], usage: Usage) {
        let w = &mut self.words;
        match usage {
            Usage::Light => {
                w[32] = p[51];
                w[28] = p[52];
                if p[48] & 0x200 == 0 {
                    w[28] &= 0x00ffffff;
                }
                w[24] = p[61];
                w[25] = p[60];
                if p[48] & 0x1400 != 0 {
                    w[24] = p[24];
                }
                w[34..48].copy_from_slice(&p[34..48]);
            }
            Usage::PostLight => {
                w[24..48].copy_from_slice(&p[24..48]);
                if p[48] & 0x400 != 0 {
                    w[31] = 0xff808080;
                }
            }
            Usage::PostLight2 => {
                w[30] = p[53];
                w[26] = p[62];
            }
            Usage::Other => w[24..48].copy_from_slice(&p[24..48]),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Reader<'a>(&'a [u8]);
    impl Reader<'_> {
        fn word(&mut self) -> u32 {
            let value = u32::from_le_bytes(self.0[..4].try_into().unwrap());
            self.0 = &self.0[4..];
            value
        }
        fn words<const N: usize>(&mut self) -> [u32; N] {
            std::array::from_fn(|_| self.word())
        }
    }
    #[test]
    fn original_material_setup_and_animated_updates_match_cpp() {
        let mut r = Reader(include_bytes!("../../tests/fixtures/material.bin"));
        assert_eq!(r.word(), 256);
        for case in 0..256 {
            let p: [u32; 64] = r.words();
            let textures = r
                .words::<8>()
                .map(|v| (v != u32::MAX).then_some(v as usize));
            let count = r.word();
            assert_eq!(r.word(), u32::from(p[48] & 0x1e != 0));
            let mut passes = Vec::new();
            for _ in 0..count {
                let usage = Usage::from_raw(r.word());
                let program = r.word();
                assert!((1..=4).contains(&program));
                assert_eq!(r.word(), p[49]);
                let size = r.word();
                let aspect = r.word();
                let state = State::build(&p, textures, usage);
                assert_eq!(state.words, r.words::<48>(), "setup case {case}");
                let expected = r
                    .words::<4>()
                    .map(|v| (v != u32::MAX).then_some(v as usize));
                assert_eq!(state.textures, expected, "textures case {case}");
                passes.push((usage, program, size, aspect, state));
            }
            let animated = r.words::<64>();
            assert_eq!(r.word(), count);
            r.word();
            for (usage, program, mut size, mut aspect, mut state) in passes {
                state.execute(&animated, usage);
                if usage == Usage::Other && animated[48] & 0x30000 != 0 {
                    size = animated[60];
                    aspect = animated[62];
                }
                assert_eq!(Usage::from_raw(r.word()), usage);
                assert_eq!(r.word(), program);
                assert_eq!(r.word(), animated[49]);
                assert_eq!(r.word(), size);
                assert_eq!(r.word(), aspect);
                assert_eq!(state.words, r.words::<48>(), "execution case {case}");
                let expected = r
                    .words::<4>()
                    .map(|v| (v != u32::MAX).then_some(v as usize));
                assert_eq!(state.textures, expected);
            }
        }
        assert!(r.0.is_empty());
    }
}
