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
/// World-space listener. Changes become audible at the next 3D commit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Listener {
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub up: [f32; 3],
    pub forward: [f32; 3],
    pub doppler: f32,
    pub rolloff: f32,
}
impl Default for Listener {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            velocity: [0.0; 3],
            up: [0.0, 1.0, 0.0],
            forward: [0.0, 0.0, 1.0],
            doppler: 1.0,
            rolloff: 1.0,
        }
    }
}
impl Listener {
    fn valid(&self) -> bool {
        self.position
            .iter()
            .chain(&self.velocity)
            .chain(&self.up)
            .chain(&self.forward)
            .all(|v| v.is_finite())
            && (0.0..=10.0).contains(&self.doppler)
            && (0.0..=10.0).contains(&self.rolloff)
    }
    /// Relative radial-velocity Doppler. The 360 units/s constant and half-
    /// speed clamp match Wine's published native DirectSound observations.
    /// The game's distance factor is always 1.
    pub fn doppler_ratio(&self, source: &Spatial) -> f64 {
        if !self.valid() || !source.valid() || self.doppler == 0.0 {
            return 1.0;
        }
        let direction = std::array::from_fn::<_, 3, _>(|i| {
            f64::from(source.position[i]) - f64::from(self.position[i])
        });
        let distance = direction.iter().map(|v| v * v).sum::<f64>().sqrt();
        if distance == 0.0 {
            return 1.0;
        }
        let relative = direction
            .iter()
            .enumerate()
            .map(|(i, d)| d * (f64::from(source.velocity[i]) - f64::from(self.velocity[i])))
            .sum::<f64>()
            / distance
            * f64::from(self.doppler);
        360.0 / (360.0 + relative.clamp(-180.0, 180.0))
    }
    /// Portable equal-power stereo rendering in the original left-handed
    /// coordinates. This replaces device-dependent DirectSound speaker routing;
    /// it does not claim sample parity with a particular Windows audio driver.
    pub fn stereo_gains(&self, source: &Spatial) -> [f32; 2] {
        let gain = self.distance_gain(source);
        if gain == 0.0 {
            return [0.0; 2];
        }
        let up = self.up.map(f64::from);
        let front = self.forward.map(f64::from);
        let right = [
            up[1] * front[2] - up[2] * front[1],
            up[2] * front[0] - up[0] * front[2],
            up[0] * front[1] - up[1] * front[0],
        ];
        let direction = std::array::from_fn::<_, 3, _>(|i| {
            f64::from(source.position[i]) - f64::from(self.position[i])
        });
        let dot = |a: [f64; 3], b: [f64; 3]| a.into_iter().zip(b).map(|(a, b)| a * b).sum::<f64>();
        let length = (dot(right, right) * dot(direction, direction)).sqrt();
        let pan = if length > 0.0 {
            (dot(right, direction) / length).clamp(-1.0, 1.0)
        } else {
            0.0
        };
        [
            ((1.0 - pan) * 0.5).sqrt() as f32 * gain,
            ((1.0 + pan) * 0.5).sqrt() as f32 * gain,
        ]
    }
    /// Inverse-distance amplitude with clamped source distances. The game
    /// creates buffers without MUTE3DATMAXDISTANCE, so far sounds remain audible.
    /// https://learn.microsoft.com/en-us/previous-versions/windows/desktop/ee418662(v=vs.85)
    pub fn distance_gain(&self, source: &Spatial) -> f32 {
        if !self.valid() || !source.valid() {
            return 0.0;
        }
        let distance = source
            .position
            .iter()
            .zip(self.position)
            .map(|(&a, b)| {
                let d = f64::from(a) - f64::from(b);
                d * d
            })
            .sum::<f64>()
            .sqrt()
            .clamp(f64::from(source.near), f64::from(source.far));
        let near = f64::from(source.near);
        (near / (near + (distance - near) * f64::from(self.rolloff))) as f32
    }
}
impl Spatial {
    fn valid(&self) -> bool {
        self.position
            .iter()
            .chain(&self.velocity)
            .all(|v| v.is_finite())
            && self.near.is_finite()
            && self.far.is_finite()
            && self.near > 0.0
            && self.far >= self.near
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Voice {
    pub started: i32,
    pub position: usize,
    /// Q32 fractional source-frame position used by the portable resampler.
    pub fraction: u32,
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
    listener: Listener,
    pending_listener: Option<Listener>,
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
            sound_time: 1,
            listener: Listener::default(),
            pending_listener: None,
        }
    }
    pub fn listener(&self) -> &Listener {
        &self.listener
    }
    pub fn set_listener(&mut self, listener: Listener) -> bool {
        if !listener.valid() {
            return false;
        }
        self.pending_listener = Some(listener);
        true
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
        voice.fraction = 0;
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
        if !parameters.valid() {
            return false;
        }
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
        self.mix_with_rates(output, |source| (spatial_gain(source), 1.0));
    }
    pub fn mix_spatial(&mut self, output: &mut [[f32; 2]]) {
        let listener = self.listener;
        self.mix_with_rates(output, |source| {
            (
                listener.stereo_gains(source),
                listener.doppler_ratio(source),
            )
        });
    }
    fn mix_with_rates(
        &mut self,
        output: &mut [[f32; 2]],
        spatial: impl Fn(&Spatial) -> ([f32; 2], f64),
    ) {
        for sample in self.samples.iter_mut().flatten() {
            for voice in &mut sample.voices[..sample.count] {
                if !voice.playing {
                    continue;
                }
                let volume = attenuation(voice.volume);
                let (gains, ratio) = match (sample.data, voice.spatial.as_ref()) {
                    (Data::Mono(_), Some(parameters)) => spatial(parameters),
                    (Data::Mono(_), None) => ([1.0; 2], 1.0),
                    (Data::Stereo(_), _) => (
                        [
                            attenuation(-voice.pan.max(0)),
                            attenuation(voice.pan.min(0)),
                        ],
                        1.0,
                    ),
                };
                let gains = gains.map(|gain| gain * volume / 32768.0);
                let step = (ratio * 4294967296.0).round() as u64;
                if step != 1u64 << 32 || voice.fraction != 0 {
                    for frame in output.iter_mut() {
                        if voice.position >= sample.data.len() {
                            break;
                        }
                        let source = interpolated(sample.data, voice.position, voice.fraction);
                        frame[0] += source[0] * gains[0];
                        frame[1] += source[1] * gains[1];
                        let phase = u64::from(voice.fraction) + step;
                        voice.position += (phase >> 32) as usize;
                        voice.fraction = phase as u32;
                    }
                    if voice.position >= sample.data.len() {
                        voice.position = sample.data.len();
                        voice.fraction = 0;
                        voice.playing = false;
                    }
                    continue;
                }
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
        if let Some(listener) = self.pending_listener.take() {
            self.listener = listener;
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

// Linear interpolation is a portable rendering choice; native driver resampling
// kernels remain a separate fidelity comparison. Unit-rate voices retain the
// exact integer PCM path above.
fn interpolated(data: &Data, index: usize, fraction: u32) -> [f32; 2] {
    let frame = |index: usize| match data {
        Data::Mono(data) => [f32::from(data[index]); 2],
        Data::Stereo(data) => data[index].map(f32::from),
    };
    let a = frame(index);
    let b = frame((index + 1).min(data.len() - 1));
    let fraction = fraction as f32 * (1.0 / 4294967296.0);
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * fraction)
}
