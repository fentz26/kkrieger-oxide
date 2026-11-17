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
#[derive(Clone, Copy)]
pub struct Parameters {
    pub attack: f32,
    pub decay: f32,
    pub sustain_level: f32,
    pub sustain_rate: f32,
    pub release: f32,
    pub volume: f32,
}
impl Envelope {
    pub fn set(&mut self, parameters: Parameters) {
        let attack = parameters.attack * -0.09375;
        self.attack_delta = power(attack + 7.);
        let decay = 1. - parameters.decay * (1. / 128.);
        self.decay_factor = 1. - power((decay - 1.) * 11.);
        self.sustain_level = parameters.sustain_level;
        self.sustain_factor = power((parameters.sustain_rate - 64.) * 0.0019375);
        let release = 1. - parameters.release * (1. / 128.);
        self.release_factor = 1. - power((release - 1.) * 11.);
        self.gain = parameters.volume * (1. / 128.);
    }

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

// The x87 helper retains transcendental precision until its final f32 store.
// exp2f can round one bit differently from that path.
fn power(value: f32) -> f32 {
    f64::from(value).exp2() as f32
}
