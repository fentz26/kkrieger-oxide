//! Original VFX0 effect entries, precalculation ramps and registered PCM data.
use crate::v2m::{
    Error, Song,
    conversion::convert,
    playback::{Playback, Synth},
};
pub const MAX_SAMPLES: usize = 0x10000;
#[derive(Clone, Copy, Debug)]
pub struct Entry {
    pub major: u8,
    pub slot: u8,
    pub gain: i8,
    pub reserved: u8,
    pub start: u32,
    pub end: u32,
    pub loop_size: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct Plan {
    pub seek: u32,
    pub frames: usize,
    pub gain: f32,
    pub fade: usize,
    pub slot: u8,
    pub spatial: bool,
}
pub struct Bank<'a> {
    pub song: &'a [u8],
    pub sample_rate: u32,
    entries: &'a [u8],
}
fn word(data: &[u8], position: usize) -> Result<u32, Error> {
    let end = position
        .checked_add(4)
        .ok_or(Error("VFX offset overflow"))?;
    Ok(u32::from_le_bytes(
        data.get(position..end)
            .ok_or(Error("truncated VFX header"))?
            .try_into()
            .unwrap(),
    ))
}
impl<'a> Bank<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self, Error> {
        if data.get(..4) != Some(b"VFX0") {
            return Err(Error("invalid VFX signature"));
        }
        let end = 8usize
            .checked_add(word(data, 4)? as usize)
            .ok_or(Error("VFX song size overflow"))?;
        let song = data.get(8..end).ok_or(Error("truncated VFX song"))?;
        let sample_rate = word(data, end)?;
        let count_position = end
            .checked_add(4)
            .ok_or(Error("VFX table offset overflow"))?;
        let count = word(data, count_position)? as usize;
        let start = end
            .checked_add(8)
            .ok_or(Error("VFX table offset overflow"))?;
        let bytes = count
            .checked_mul(16)
            .and_then(|n| start.checked_add(n))
            .ok_or(Error("VFX table size overflow"))?;
        let entries = data
            .get(start..bytes)
            .ok_or(Error("truncated VFX entry table"))?;
        Ok(Self {
            song,
            sample_rate,
            entries,
        })
    }
    pub fn len(&self) -> usize {
        self.entries.len() / 16
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn entries(&self) -> impl ExactSizeIterator<Item = Entry> + '_ {
        self.entries.as_chunks::<16>().0.iter().map(|b| Entry {
            major: b[0],
            slot: b[1],
            gain: b[2] as i8,
            reserved: b[3],
            start: u32::from_le_bytes(b[4..8].try_into().unwrap()),
            end: u32::from_le_bytes(b[8..12].try_into().unwrap()),
            loop_size: u32::from_le_bytes(b[12..16].try_into().unwrap()),
        })
    }
    pub fn render(&self, seed: u32) -> Result<Vec<Sample>, Error> {
        if self.sample_rate == 0 {
            return Err(Error("zero VFX recording rate"));
        }
        let converted = convert(self.song)?;
        let song = Song::parse(&converted)?;
        let synth = Synth::new(&song, 44100, seed)?;
        // VFX positions are samples at the rate recorded after the embedded song.
        let mut player = Playback::new(&song, 44100, self.sample_rate, synth)?;
        let mut audio = vec![[0.; 2]; MAX_SAMPLES];
        let mut pcm = vec![[0i16; 2]; MAX_SAMPLES];
        let mut registered = [false; 256];
        let mut samples = Vec::with_capacity(self.len());
        for entry in self.entries() {
            let plan = entry.plan()?;
            player.play(plan.seek)?;
            player.render(&mut audio[..plan.frames], plan.frames)?;
            plan.encode(&audio[..plan.frames], &mut pcm[..plan.frames])?;
            if registered[usize::from(plan.slot)] || plan.frames == 0 {
                continue;
            }
            registered[usize::from(plan.slot)] = true;
            let data = if plan.spatial {
                Data::Mono(
                    pcm[..plan.frames]
                        .iter()
                        .map(|&frame| downmix(frame))
                        .collect(),
                )
            } else {
                Data::Stereo(pcm[..plan.frames].into())
            };
            samples.push(Sample {
                slot: plan.slot,
                buffers: 4,
                data,
            });
        }
        player.close()?;
        Ok(samples)
    }
}
impl Entry {
    pub fn plan(self) -> Result<Plan, Error> {
        let start = if self.loop_size != 0 {
            self.end.wrapping_sub(self.loop_size)
        } else {
            self.start
        };
        let count = self.end.wrapping_sub(start) as i32;
        if count < 0 {
            return Err(Error("negative VFX sample span"));
        }
        let frames = (count as usize).min(MAX_SAMPLES);
        Ok(Plan {
            seek: self.start,
            frames,
            gain: 10.0f64.powf(f64::from((f32::from(self.gain) / 5.) / 20.)) as f32,
            fade: if self.major & 1 != 0 { 44100 } else { 1000 },
            slot: self.slot,
            spatial: self.major & 1 == 0,
        })
    }
}
impl Plan {
    pub fn encode(self, input: &[[f32; 2]], output: &mut [[i16; 2]]) -> Result<(), Error> {
        if input.len() < self.frames || output.len() < self.frames {
            return Err(Error("VFX PCM buffer too short"));
        }
        if input[..self.frames]
            .iter()
            .flatten()
            .any(|v| !v.is_finite())
        {
            return Err(Error("non-finite VFX source sample"));
        }
        for (i, (source, dest)) in input.iter().zip(output).take(self.frames).enumerate() {
            let mut gain = self.gain;
            if i < self.fade {
                gain = (gain * i as f32) / self.fade as f32;
            }
            if i as i32 > self.frames as i32 - self.fade as i32 {
                gain = (gain * (self.frames - i) as f32) / self.fade as f32;
            }
            *dest = source.map(|v| (((v * gain) * 32767.) as i32).clamp(-32767, 32767) as i16);
        }
        Ok(())
    }
}
pub fn downmix(frame: [i16; 2]) -> i16 {
    ((i32::from(frame[0]) + i32::from(frame[1])) >> 1) as i16
}
pub enum Data {
    Mono(Box<[i16]>),
    Stereo(Box<[[i16; 2]]>),
}
pub struct Sample {
    pub slot: u8,
    pub buffers: u8,
    pub data: Data,
}
impl Data {
    pub fn len(&self) -> usize {
        match self {
            Self::Mono(v) => v.len(),
            Self::Stereo(v) => v.len(),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
