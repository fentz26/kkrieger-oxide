//! Original 32-voice channel allocation and unsigned-age stealing policy.
pub const VOICES: usize = 32;
pub struct Allocator {
    pub channels: [i32; VOICES],
    pub ages: [u32; VOICES],
    pub latest: [u32; 16],
    pub counter: u32,
}
impl Default for Allocator {
    fn default() -> Self {
        Self {
            channels: [-1; VOICES],
            ages: [0; VOICES],
            latest: [0; 16],
            counter: 0,
        }
    }
}
impl Allocator {
    fn oldest(&self, channel: Option<i32>, released: bool, gates: &[u32; VOICES]) -> Option<usize> {
        let mut age = self.counter;
        let mut selected = None;
        for (i, &gate) in gates.iter().enumerate() {
            if channel.is_some_and(|channel| self.channels[i] != channel) || (released && gate != 0)
            {
                continue;
            }
            if self.ages[i] < age {
                age = self.ages[i];
                selected = Some(i);
            }
        }
        selected
    }
    pub fn allocate(
        &mut self,
        channel: u8,
        maximum_polyphony: u8,
        gates: &[u32; VOICES],
    ) -> Result<usize, &'static str> {
        if channel >= 16 {
            return Err("V2 MIDI channel outside 0..16");
        }
        let channel = i32::from(channel);
        let count = self.channels.iter().filter(|&&c| c == channel).count();
        let selected = if count >= usize::from(maximum_polyphony) {
            self.oldest(Some(channel), true, gates)
                .or_else(|| self.oldest(Some(channel), false, gates))
        } else {
            self.channels
                .iter()
                .position(|&c| c < 0)
                .or_else(|| self.oldest(None, true, gates))
                .or_else(|| self.oldest(None, false, gates))
        }
        .ok_or("V2 allocation age counter has no older voice")?;
        self.channels[selected] = channel;
        self.latest[channel as usize] = selected as u32;
        self.ages[selected] = self.counter;
        self.counter = self.counter.wrapping_add(1);
        Ok(selected)
    }
}
