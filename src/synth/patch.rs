//! Borrowed original V2 sound bank and ordered voice/channel modulation matrix.
use super::voice::Voice;
pub const VOICE_VALUES: usize = 59;
pub const CHANNEL_VALUES: usize = 29;
const PARAMETER_VALUES: usize = VOICE_VALUES + CHANNEL_VALUES;
#[derive(Clone, Copy)]
pub struct Bank<'a>(&'a [u8]);
impl<'a> Bank<'a> {
    pub fn new(data: &'a [u8]) -> Result<Self, &'static str> {
        let first = data.get(..4).ok_or("truncated V2 patch map")?;
        let table_size = u32::from_le_bytes(first.try_into().unwrap()) as usize;
        if table_size == 0
            || !table_size.is_multiple_of(4)
            || table_size > 128 * 4
            || table_size > data.len()
        {
            return Err("invalid V2 patch map size");
        }
        Ok(Self(data))
    }
    pub fn program_count(&self) -> usize {
        u32::from_le_bytes(self.0[..4].try_into().unwrap()) as usize / 4
    }
    /// The bank must use the current synth layout; convert legacy V2M data first.
    pub fn patch(&self, program: u8) -> Result<Patch<'a>, &'static str> {
        if usize::from(program) >= self.program_count() {
            return Err("V2 program outside patch map");
        }
        let i = usize::from(program) * 4;
        let offset = u32::from_le_bytes(self.0[i..i + 4].try_into().unwrap()) as usize;
        Patch::parse(self.0.get(offset..).ok_or("V2 patch offset outside bank")?)
    }
}
#[derive(Clone, Copy)]
pub struct Patch<'a> {
    values: &'a [u8],
    modulations: &'a [u8],
    pub maximum_polyphony: u8,
}
impl<'a> Patch<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self, &'static str> {
        let header = data
            .get(..PARAMETER_VALUES + 2)
            .ok_or("truncated V2 patch header")?;
        let count = usize::from(header[PARAMETER_VALUES + 1]);
        let modulations = data
            .get(PARAMETER_VALUES + 2..PARAMETER_VALUES + 2 + count * 3)
            .ok_or("truncated V2 modulation matrix")?;
        Ok(Self {
            values: &header[..PARAMETER_VALUES],
            modulations,
            maximum_polyphony: header[PARAMETER_VALUES],
        })
    }
    pub fn voice_values(&self, voice: &Voice, controls: &[u8; 7]) -> [f32; VOICE_VALUES] {
        self.modulate::<VOICE_VALUES>(0, voice, controls)
    }
    pub fn channel_values(&self, voice: &Voice, controls: &[u8; 7]) -> [f32; CHANNEL_VALUES] {
        self.modulate::<CHANNEL_VALUES>(VOICE_VALUES, voice, controls)
    }
    fn modulate<const N: usize>(
        &self,
        start: usize,
        voice: &Voice,
        controls: &[u8; 7],
    ) -> [f32; N] {
        let mut values = std::array::from_fn(|i| f32::from(self.values[start + i]));
        for entry in self.modulations.as_chunks::<3>().0 {
            let destination = usize::from(entry[2]);
            if !(start..start + N).contains(&destination) {
                continue;
            }
            let source = match entry[0] {
                0 => voice.velocity,
                1..=7 => f32::from(controls[usize::from(entry[0]) - 1]),
                8 => voice.envelopes[0].output,
                9 => voice.envelopes[1].output,
                10 => voice.lfos[0].output,
                11 => voice.lfos[1].output,
                // The original falls through to note tracking for every >=12 index.
                _ => {
                    let note = (f64::from(voice.note) - 48.) as f32;
                    note + note
                }
            };
            let amount = (f32::from(entry[1]) - 64.) * (1. / 128.);
            let amount = amount + amount;
            let value = source * amount + values[destination - start];
            // Original signed-negative test followed by an unsigned float-bit clamp.
            let bits = value.to_bits();
            values[destination - start] = f32::from_bits(if bits & 0x80000000 != 0 {
                0
            } else {
                bits.min(0x43000000)
            });
        }
        values
    }
}
