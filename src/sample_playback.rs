//! Original sample registration, rotating playback slots and 3D token lifetime.
use crate::vfx::{Data, Sample};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spatial {
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub near: f32,
    pub far: f32,
}
impl Default for Spatial {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            velocity: [0.0; 3],
            near: 1.0,
            far: 1_000_000_000.0,
        }
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Voice {
    pub started: i32,
    pub position: usize,
    pub playing: bool,
    /// Original volume and pan in hundredths of a decibel.
    pub volume: i32,
    pub pan: i32,
    pub spatial: Option<Spatial>,
}
pub struct Registered<'a> {
    pub data: &'a Data,
    pub voices: [Voice; 8],
    pub count: usize,
    pub uses: i32,
    pub last: usize,
    pub length_ms: i32,
}
pub struct Pool<'a> {
    samples: Box<[Option<Registered<'a>>]>,
    pub sound_time: i32,
}
impl Default for Pool<'_> {
    fn default() -> Self {
        Self::new()
    }
}
impl<'a> Pool<'a> {
    pub fn new() -> Self {
        Self {
            samples: (0..256).map(|_| None).collect(),
            sound_time: 0,
        }
    }
    pub fn sample(&self, slot: u8) -> Option<&Registered<'a>> {
        self.samples[usize::from(slot)].as_ref()
    }
    pub fn add(&mut self, sample: &'a Sample) -> bool {
        let slot = &mut self.samples[usize::from(sample.slot)];
        let length_ms = sample.data.len().checked_mul(1000).map(|n| n / 44100 + 10);
        let Some(length_ms) = length_ms.and_then(|n| i32::try_from(n).ok()) else {
            return false;
        };
        if slot.is_some() || !(1..=8).contains(&sample.buffers) || sample.data.is_empty() {
            return false;
        }
        *slot = Some(Registered {
            data: &sample.data,
            voices: [Voice::default(); 8],
            count: usize::from(sample.buffers),
            uses: 0,
            last: 0,
            length_ms,
        });
        true
    }
    pub fn remove(&mut self, slot: u8) {
        self.samples[usize::from(slot)] = None;
    }
    pub fn play(&mut self, slot: u8, volume: f32, pan: f32) -> Option<i32> {
        if !volume.is_finite() || !pan.is_finite() {
            return None;
        }
        let sample = self.samples[usize::from(slot)].as_mut()?;
        sample.last = (sample.last + 1) % sample.count;
        let voice = &mut sample.voices[sample.last];
        voice.started = self.sound_time;
        voice.position = 0;
        voice.playing = true;
        let requested_volume = (f64::from(volume.clamp(1e-20, 1.0)).ln() * 20.0 * 100.0) as i32;
        // SetVolume rejects values outside [-10000, 0]; the original ignores
        // that failure, so a reused buffer keeps its previous volume.
        if (-10000..=0).contains(&requested_volume) {
            voice.volume = requested_volume;
        }
        if matches!(sample.data, Data::Stereo(_)) {
            voice.pan = ((pan * 100.0) as i32).clamp(-10000, 10000);
        }
        // Playing again does not reset a buffer's existing 3D parameters.
        sample.uses = sample.uses.wrapping_add(1);
        Some(sample.uses)
    }
    pub fn spatial(&mut self, slot: u8, token: i32, parameters: Spatial) -> bool {
        let Some(sample) = self.samples[usize::from(slot)].as_mut() else {
            return false;
        };
        if !matches!(sample.data, Data::Mono(_))
            || sample.uses.wrapping_sub(token) >= sample.count as i32
        {
            return false;
        }
        let index = token % sample.count as i32;
        // The original indexes before its allocation for negative tokens.
        let Ok(index) = usize::try_from(index) else {
            return false;
        };
        let voice = &mut sample.voices[index];
        if voice.started == 0 {
            return false;
        }
        voice.spatial = Some(parameters);
        true
    }
    /// Add one-shot PCM at its original 44,100 Hz rate. The caller supplies
    /// listener-dependent gains for buffers with active 3D parameters.
    /// Stereo panning and volume use DirectSound's hundredth-decibel units.
    pub fn mix(&mut self, output: &mut [[f32; 2]], spatial_gain: impl Fn(&Spatial) -> [f32; 2]) {
        for sample in self.samples.iter_mut().flatten() {
            for voice in &mut sample.voices[..sample.count] {
                if !voice.playing {
                    continue;
                }
                let volume = attenuation(voice.volume);
                let gains = match (sample.data, voice.spatial.as_ref()) {
                    (Data::Mono(_), Some(parameters)) => spatial_gain(parameters),
                    (Data::Mono(_), None) => [1.0; 2],
                    (Data::Stereo(_), _) => [
                        attenuation(-voice.pan.max(0)),
                        attenuation(voice.pan.min(0)),
                    ],
                }
                .map(|gain| gain * volume / 32768.0);
                let frames = output.len().min(sample.data.len() - voice.position);
                match sample.data {
                    Data::Mono(data) => {
                        for (frame, &source) in
                            output[..frames].iter_mut().zip(&data[voice.position..])
                        {
                            frame[0] += f32::from(source) * gains[0];
                            frame[1] += f32::from(source) * gains[1];
                        }
                    }
                    Data::Stereo(data) => {
                        for (frame, source) in
                            output[..frames].iter_mut().zip(&data[voice.position..])
                        {
                            frame[0] += f32::from(source[0]) * gains[0];
                            frame[1] += f32::from(source[1]) * gains[1];
                        }
                    }
                }
                voice.position += frames;
                voice.playing = voice.position != sample.data.len();
            }
        }
    }
    pub fn commit(&mut self, now: i32) {
        for sample in self.samples.iter_mut().flatten() {
            if matches!(sample.data, Data::Mono(_)) {
                for voice in &mut sample.voices[..sample.count] {
                    if voice.started != 0
                        && voice.started.wrapping_add(sample.length_ms) < self.sound_time
                    {
                        voice.started = 0;
                        voice.spatial = None;
                    }
                }
            }
        }
        // Native expiry deliberately uses the preceding commit's clock.
        self.sound_time = now;
    }
}

// DirectSound controls:
// https://learn.microsoft.com/en-us/previous-versions/windows/desktop/ee418150(v=vs.85)
// https://learn.microsoft.com/en-us/previous-versions/windows/desktop/ee418148(v=vs.85)
fn attenuation(hundredths_db: i32) -> f32 {
    if hundredths_db <= -10000 {
        0.0
    } else {
        10.0_f64.powf(f64::from(hundredths_db) / 2000.0) as f32
    }
}
