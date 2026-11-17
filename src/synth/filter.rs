//! Original two-times-oversampled ViruZ II state-variable filter.
#[derive(Clone, Copy)]
pub struct Parameters {
    pub mode: f32,
    pub cutoff: f32,
    pub resonance: f32,
}
#[derive(Clone, Copy)]
pub struct Filter {
    pub mode: u32,
    pub frequency: f32,
    pub resonance: f32,
    pub low: f32,
    pub band: f32,
    pub step_bytes: u32,
}
impl Default for Filter {
    fn default() -> Self {
        Self {
            mode: 0,
            frequency: 0.,
            resonance: 0.,
            low: 0.,
            band: 0.,
            step_bytes: 4,
        }
    }
}
impl Filter {
    pub fn init(&mut self) {
        self.low = 0.;
        self.band = 0.;
        self.step_bytes = 4;
    }
    pub fn set(&mut self, p: Parameters, linear_frequency: f32) {
        self.mode = super::math::integer(p.mode) as u32;
        let cutoff = p.cutoff * (1. / 128.);
        let exponent = (cutoff - 1.) * 10.;
        let frequency = f64::from(exponent).exp2() as f32;
        self.resonance = 1. - p.resonance * (1. / 128.);
        self.frequency = frequency * linear_frequency;
    }
    pub fn render(
        &mut self,
        destination: &mut [f32],
        source: &[f32],
        count: usize,
    ) -> Result<(), &'static str> {
        let mode = self.mode & 7;
        if matches!(mode, 0 | 6 | 7) {
            let source = source.get(..count).ok_or("filter source too short")?;
            destination
                .get_mut(..count)
                .ok_or("filter destination too short")?
                .copy_from_slice(source);
            return Ok(());
        }
        let stride = self.check(count, destination.len(), source.len())?;
        for i in 0..count {
            destination[i * stride] = self.sample(source[i * stride], mode);
        }
        Ok(())
    }
    pub fn render_in_place(
        &mut self,
        buffer: &mut [f32],
        count: usize,
    ) -> Result<(), &'static str> {
        let mode = self.mode & 7;
        if matches!(mode, 0 | 6 | 7) {
            return Ok(());
        }
        let stride = self.check(count, buffer.len(), buffer.len())?;
        for i in 0..count {
            let index = i * stride;
            buffer[index] = self.sample(buffer[index], mode);
        }
        Ok(())
    }
    fn check(
        &self,
        count: usize,
        destination: usize,
        source: usize,
    ) -> Result<usize, &'static str> {
        if count == 0 {
            return Ok(1);
        }
        if self.step_bytes == 0 || !self.step_bytes.is_multiple_of(4) {
            return Err("invalid filter sample stride");
        }
        let stride = self.step_bytes as usize / 4;
        let span = (count - 1)
            .checked_mul(stride)
            .and_then(|n| n.checked_add(1))
            .ok_or("filter span overflow")?;
        if span > destination || span > source {
            return Err("filter buffer too short");
        }
        Ok(stride)
    }
    fn sample(&mut self, value: f32, mode: u32) -> f32 {
        let input = value + f32::from_bits(0x36800000);
        self.low += self.band * self.frequency;
        let high = (input - self.band * self.resonance) - self.low;
        self.band += self.frequency * high;
        self.low += self.band * self.frequency;
        let high = (input - self.band * self.resonance) - self.low;
        self.band += self.frequency * high;
        match mode {
            1 => self.low,
            2 => self.band,
            3 => high,
            4 => high + self.low,
            5 => (high + self.band) + self.low,
            _ => unreachable!(),
        }
    }
}
