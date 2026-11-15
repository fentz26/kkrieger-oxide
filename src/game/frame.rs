//! Original player main-loop timing prefix, before OnTick and InitFrame.
pub struct Clock {
    pub first_time: i32,
    pub last_time: i32,
    pub smoothed_ms: f32,
}
pub struct Frame {
    pub time: i32,
    pub beat: i32,
    pub slices: i32,
    pub milliseconds: i32,
    pub fps: f32,
}
impl Clock {
    pub fn new(first_time: i32) -> Self {
        Self {
            first_time,
            last_time: 0,
            smoothed_ms: 0.,
        }
    }
    pub fn advance(&mut self, now: i32, bpm: u32) -> Frame {
        let time = now.wrapping_sub(self.first_time);
        let beat = ((i64::from(time) * i64::from(bpm as i32)) / 60000) as i32;
        let milliseconds = time.wrapping_sub(self.last_time);
        self.smoothed_ms += (milliseconds as f32 - self.smoothed_ms) * 0.1;
        let slices = (time / 10).wrapping_sub(self.last_time / 10).min(10);
        self.last_time = time;
        Frame {
            time,
            beat,
            slices,
            milliseconds,
            fps: 1000. / self.smoothed_ms,
        }
    }
}
