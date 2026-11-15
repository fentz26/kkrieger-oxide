//! Persistent envelope used by the original Misc_Delay execution handler.
//! Instance allocation and child execution belong to the scene interpreter.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct State {
    pub phase: i32,
    pub time: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    pub flags: u32,
    pub threshold: f32,
    pub hysteresis: f32,
    pub durations: [f32; 3],
}

impl State {
    /// Advance one original Exec call. `switch` is the selected game switch;
    /// flags choose between that switch and one of the first four VM variables.
    pub fn advance(
        &mut self,
        p: Parameters,
        switch: u8,
        variables: &mut [[f32; 4]],
        current_time: i32,
        last_time: i32,
    ) {
        let (value, threshold, hysteresis) = if p.flags & 0xc0 != 0 {
            (
                variables[((p.flags & 0xc0) >> 6) as usize][0],
                p.threshold,
                p.hysteresis,
            )
        } else {
            (switch as f32, 0.5, 0.)
        };
        if self.phase == 2 && value < threshold - hysteresis {
            self.phase = 3;
            self.time = 0.;
        }
        if self.phase == 0 && value > threshold + hysteresis {
            self.phase = 1;
            self.time = 0.;
        }
        let mut delta = current_time.wrapping_sub(last_time) as f32 / 1000.;
        if delta < 0. {
            delta = 0.;
        }
        match self.phase {
            1 | 3 => {
                let duration = p.durations[if self.phase == 1 { 0 } else { 2 }];
                if duration > 0. {
                    self.time += delta / duration;
                } else {
                    self.time = 1.;
                }
                if self.time >= 1. {
                    self.phase = if self.phase == 1 { 2 } else { 0 };
                    self.time = 0.;
                }
            }
            2 if p.durations[1] > 0. => {
                self.time += delta / p.durations[1];
                if self.time > 1. {
                    self.time -= 1.;
                }
                if self.time > 1. {
                    self.time = 0.;
                }
            }
            _ => {}
        }
        let mut value = 0.;
        if self.phase == 1 {
            value = self.time;
        }
        if self.phase == 2 {
            value = 1.;
        }
        match p.flags & 0xf {
            0 => {
                if self.phase == 3 {
                    value = 1. - self.time;
                }
            }
            1 => {
                if self.phase == 3 {
                    value = 1. + self.time;
                }
                value *= 0.5;
            }
            2 => {
                if self.phase == 2 {
                    value = 1. + self.time * 2.;
                }
                if self.phase == 3 {
                    value = 3. + self.time;
                }
                value *= 0.25;
            }
            _ => {}
        }
        variables[((p.flags & 0x30) >> 4) as usize] = [value; 4];
    }
}
