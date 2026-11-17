//! Original MIDI controller/program state and voice/speech actions.
use super::allocation::{Allocator, VOICES};
#[derive(Clone, Copy)]
pub struct Channel {
    pub program: u8,
    pub controls: [u8; 7],
}
impl Default for Channel {
    fn default() -> Self {
        Self {
            program: 0,
            controls: [0, 0, 0, 0, 0, 0, 127],
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Speech {
    Controller { index: u8, value: u8 },
    Release,
}
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Changes {
    pub reset: u32,
    pub release: u32,
    pub speech: Option<Speech>,
}
pub struct State {
    pub channels: [Channel; 16],
    pub sample_rate: u32,
}
impl State {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            channels: [Channel::default(); 16],
            sample_rate,
        }
    }
    pub fn program(
        &mut self,
        channel: u8,
        program: u8,
        allocator: &mut Allocator,
    ) -> Result<(), &'static str> {
        let info = self
            .channels
            .get_mut(usize::from(channel))
            .ok_or("V2 MIDI channel outside 0..16")?;
        let program = program & 127;
        if info.program != program {
            info.program = program;
            for mapped in &mut allocator.channels {
                if *mapped == i32::from(channel) {
                    *mapped = -1;
                }
            }
        }
        // This also runs for repeated program changes, leaving controller 7 alone.
        info.controls[..6].fill(0);
        Ok(())
    }
    pub fn control(
        &mut self,
        channel: u8,
        controller: u8,
        value: u8,
        allocator: &mut Allocator,
    ) -> Result<Changes, &'static str> {
        if channel >= 16 {
            return Err("V2 MIDI channel outside 0..16");
        }
        let mut changes = Changes::default();
        match controller {
            1..=7 => {
                self.channels[usize::from(channel)].controls[usize::from(controller) - 1] = value;
                if channel == 15 {
                    changes.speech = Some(Speech::Controller {
                        index: controller,
                        value,
                    });
                }
            }
            120 | 123..=255 => {
                if controller != 120 && channel == 15 {
                    changes.speech = Some(Speech::Release);
                }
                for voice in 0..VOICES {
                    // Native chanmap[-1] aliases the preceding samplerate word.
                    // These loops deliberately target the following voice.
                    let mapped = if voice == 0 {
                        self.sample_rate
                    } else {
                        allocator.channels[voice - 1] as u32
                    };
                    if mapped as u8 != channel {
                        continue;
                    }
                    if controller == 120 {
                        changes.reset |= 1 << voice;
                        if voice == 0 {
                            self.sample_rate = u32::MAX;
                        } else {
                            allocator.channels[voice - 1] = -1;
                        }
                    } else {
                        changes.release |= 1 << voice;
                    }
                }
            }
            // The original controller-121 reset handler has no implementation.
            _ => {}
        }
        Ok(changes)
    }
    pub fn note_off(
        &self,
        channel: u8,
        note: u8,
        allocator: &Allocator,
        notes: &[i32; VOICES],
        gates: &[u32; VOICES],
    ) -> Result<Changes, &'static str> {
        if channel >= 16 {
            return Err("V2 MIDI channel outside 0..16");
        }
        let mut changes = Changes {
            speech: if channel == 15 {
                Some(Speech::Release)
            } else {
                None
            },
            ..Changes::default()
        };
        for voice in 0..VOICES {
            if allocator.channels[voice] == i32::from(channel)
                && notes[voice] == i32::from(note)
                && gates[voice] as u8 != 0
            {
                changes.release = 1 << voice;
                break;
            }
        }
        Ok(changes)
    }
}
