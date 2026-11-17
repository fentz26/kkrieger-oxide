//! Original syEnvInit/syEnvTick, retaining unsigned float-bit comparisons.
#[derive(Clone, Copy, Default)]
pub struct Envelope {
    pub output: f32,
    pub state: u32,
    pub value: f32,
    pub attack_delta: f32,
    pub decay_factor: f32,
    pub sustain_level: f32,
    pub sustain_factor: f32,
    pub release_factor: f32,
    pub gain: f32,
}
impl Envelope {
    pub fn init(&mut self) {
        self.state = 0;
    }
    pub fn tick(&mut self, gate: bool) -> Result<f32, &'static str> {
        if self.state > 4 {
            return Err("invalid V2 envelope state");
        }
        let mut state = self.state;
        if gate && (state == 0 || state == 4) {
            state = 1;
        }
        if !gate && (1..=3).contains(&state) {
            state = 4;
        }
        self.state = state;
        match state {
            0 => self.value = 0.,
            1 => {
                self.value += self.attack_delta;
                if self.value.to_bits() >= 128f32.to_bits() {
                    self.value = 128.;
                    self.state = 2;
                }
            }
            2 => {
                self.value *= self.decay_factor;
                if self.value.to_bits() <= self.sustain_level.to_bits() {
                    self.value = self.sustain_level;
                    self.state = 3;
                } else {
                    self.check_runout();
                }
            }
            3 => {
                self.value *= self.sustain_factor;
                if self.value.to_bits() <= 0x39000000 {
                    self.value = 0.;
                    self.state = 0;
                } else if self.value.to_bits() >= 128f32.to_bits() {
                    self.value = 128.;
                }
            }
            4 => {
                self.value *= self.release_factor;
                self.check_runout();
            }
            _ => unreachable!(),
        }
        self.output = self.value * self.gain;
        Ok(self.output)
    }
    fn check_runout(&mut self) {
        if self.value.to_bits() <= 0x39000000 {
            self.value = 0.;
            self.state = 0;
        }
    }
}
