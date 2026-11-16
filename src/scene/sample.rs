//! Original Misc_PlaySample instance state and platform audio callbacks.
#[derive(Clone, Copy)]
pub struct Parameters {
    pub value: i32,
    pub threshold: f32,
    pub sample: i32,
    pub volume_db: f32,
    pub retrigger: f32,
    pub half_range: f32,
}
pub trait Output {
    fn play(&mut self, sample: i32, volume: f32) -> i32;
    fn spatial(
        &mut self,
        sample: i32,
        buffer: i32,
        position: [f32; 4],
        velocity: [f32; 4],
        near: f32,
        far: f32,
    );
}
#[derive(Clone, Debug)]
pub struct State {
    pub on: bool,
    pub retrigger: i32,
    pub old_value: u8,
    pub old_position: [f32; 4],
    pub buffer: i32,
    initialized: bool,
}
impl Default for State {
    fn default() -> Self {
        Self {
            on: false,
            retrigger: -1,
            old_value: 0,
            old_position: [0.; 4],
            buffer: -1,
            initialized: false,
        }
    }
}
impl State {
    pub fn advance<'a>(
        &mut self,
        parameters: Parameters,
        switch: u8,
        delta: i32,
        position: [f32; 4],
        mut output: Option<&mut (dyn Output + 'a)>,
    ) {
        if !self.initialized {
            self.retrigger = -1;
            self.old_value = switch;
            self.old_position = [0.; 4];
            self.buffer = -1;
            self.initialized = true;
        }
        let active = if parameters.value == -1 {
            switch != self.old_value
        } else {
            i32::from(switch) == parameters.value && parameters.threshold > 0.
        };
        self.old_value = switch;
        if self.retrigger >= 0 {
            self.retrigger = self.retrigger.wrapping_sub(delta);
            if self.retrigger < 0 {
                self.on = false;
            }
        }
        if active && !self.on {
            let volume = 10f64.powf(f64::from(-parameters.volume_db / 20.)) as f32;
            let buffer = output
                .as_deref_mut()
                .map_or(-1, |output| output.play(parameters.sample, volume));
            if parameters.retrigger > 0. {
                self.retrigger = (parameters.retrigger * 1000.).round_ties_even() as i32;
            }
            if parameters.half_range != 0. {
                self.buffer = buffer;
            }
        }
        if self.buffer != -1 {
            let scale = 1000. / (delta as f32 + 1.);
            let velocity = [
                (position[0] - self.old_position[0]) * scale,
                (position[1] - self.old_position[1]) * scale,
                (position[2] - self.old_position[2]) * scale,
                0.,
            ];
            if let Some(output) = output {
                output.spatial(
                    parameters.sample,
                    self.buffer,
                    position,
                    velocity,
                    1.,
                    parameters.half_range * 8.,
                );
            }
            self.old_position = position;
        }
        self.on = active;
    }
}
