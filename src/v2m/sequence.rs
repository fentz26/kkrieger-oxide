//! Original CV2MPlayer Reset/Tick event ordering and delta accumulators.
use super::{Error, Song, Stream};
#[derive(Clone, Copy, Default)]
pub struct Cursor {
    pub index: u32,
    pub next_time: u32,
    pub last: [u8; 2],
}
#[derive(Clone, Copy, Default)]
pub struct Channel {
    pub notes: Cursor,
    pub programs: Cursor,
    pub pitch: Cursor,
    pub controls: [Cursor; 7],
}
pub struct Sequencer<'s, 'a> {
    song: &'s Song<'a>,
    pub playing: bool,
    pub time: u32,
    pub next_time: u32,
    pub global: Cursor,
    pub channels: [Channel; 16],
    pub microseconds: u32,
    pub numerator: u32,
    pub denominator: u32,
    pub ticks_per_quarter: u32,
    pub bar: u32,
    pub beat: u32,
    pub tick: u32,
    pub midi: Vec<u8>,
    sample_rate: u32,
}
impl<'s, 'a> Sequencer<'s, 'a> {
    pub fn new(song: &'s Song<'a>, sample_rate: u32) -> Result<Self, Error> {
        let mut result = Self::initial_state(song, sample_rate)?;
        result.midi.reserve(4096);
        Ok(result)
    }
    /// Restore original Reset state while retaining the fixed MIDI workspace.
    /// Validate the replacement before changing an existing sequence.
    pub fn reset(&mut self) -> Result<(), Error> {
        let mut replacement = Self::initial_state(self.song, self.sample_rate)?;
        std::mem::swap(&mut replacement.midi, &mut self.midi);
        replacement.midi.clear();
        *self = replacement;
        Ok(())
    }
    fn initial_state(song: &'s Song<'a>, sample_rate: u32) -> Result<Self, Error> {
        let mut result = Self {
            song,
            playing: true,
            time: 0,
            next_time: u32::MAX,
            global: Cursor::default(),
            channels: [Channel::default(); 16],
            microseconds: 5000u32.wrapping_mul(sample_rate),
            numerator: 4,
            denominator: 4,
            ticks_per_quarter: 8,
            bar: 0,
            beat: 0,
            tick: 0,
            midi: Vec::new(),
            sample_rate,
        };
        if song.time_division == 0 {
            return Err(Error("zero V2M time division"));
        }
        initial(
            song,
            song.global_events,
            song.global_events.count,
            &mut result.global,
            &mut result.next_time,
        )?;
        for (base, state) in song.channels.iter().zip(&mut result.channels) {
            if base.notes.count == 0 {
                continue;
            }
            initial(
                song,
                base.notes,
                base.notes.count,
                &mut state.notes,
                &mut result.next_time,
            )?;
            initial(
                song,
                base.programs,
                base.programs.count,
                &mut state.programs,
                &mut result.next_time,
            )?;
            // Published Reset uses the program count for the pitch time planes.
            initial(
                song,
                base.pitch,
                base.programs.count,
                &mut state.pitch,
                &mut result.next_time,
            )?;
            for (stream, cursor) in base.controls.into_iter().zip(&mut state.controls) {
                initial(song, stream, stream.count, cursor, &mut result.next_time)?;
            }
        }
        Ok(result)
    }
    pub fn advance(&mut self) -> Result<(), Error> {
        self.midi.clear();
        if !self.playing {
            return Ok(());
        }
        self.tick = self
            .tick
            .wrapping_add(self.next_time.wrapping_sub(self.time));
        self.beat = self.beat.wrapping_add(self.tick / self.song.time_division);
        self.tick %= self.song.time_division;
        if self.denominator == 0 {
            return Err(Error("zero V2M time signature denominator"));
        }
        let qpb = self.numerator.wrapping_mul(4) / self.denominator;
        if qpb == 0 {
            return Err(Error("zero V2M beats per bar"));
        }
        self.bar = self.bar.wrapping_add(self.beat / qpb);
        self.beat %= qpb;
        self.time = self.next_time;
        self.next_time = u32::MAX;
        let mut last_status = None;
        let base = self.song.global_events;
        if self.global.index < base.count && self.time == self.global.next_time {
            let n = base.count as usize;
            let index = self.global.index as usize;
            let tempo =
                u32::from_le_bytes(std::array::from_fn(|i| base.data[3 * n + 4 * index + i]));
            self.microseconds = tempo.wrapping_mul(self.sample_rate / 100);
            self.numerator = u32::from(base.data[7 * n + index]);
            self.denominator = u32::from(base.data[8 * n + index]);
            self.ticks_per_quarter = u32::from(base.data[9 * n + index]);
            self.global.index += 1;
            update(self.song, base, &mut self.global, self.time)?;
        }
        consider(self.global, base.count, &mut self.next_time);
        for (channel, (base, state)) in self
            .song
            .channels
            .iter()
            .zip(&mut self.channels)
            .enumerate()
        {
            if base.notes.count == 0 {
                continue;
            }
            if due(state.programs, base.programs, self.time) {
                status(&mut self.midi, &mut last_status, 0xc0 | channel as u8)?;
                state.programs.last[0] = state.programs.last[0].wrapping_add(byte(
                    self.song,
                    base.programs,
                    state.programs.index as usize + 3 * base.programs.count as usize,
                )?);
                push(&mut self.midi, state.programs.last[0])?;
                state.programs.index += 1;
                update(self.song, base.programs, &mut state.programs, self.time)?;
            }
            consider(state.programs, base.programs.count, &mut self.next_time);
            for (control, (stream, cursor)) in base
                .controls
                .into_iter()
                .zip(&mut state.controls)
                .enumerate()
            {
                if due(*cursor, stream, self.time) {
                    status(&mut self.midi, &mut last_status, 0xb0 | channel as u8)?;
                    push(&mut self.midi, control as u8 + 1)?;
                    cursor.last[0] = cursor.last[0].wrapping_add(byte(
                        self.song,
                        stream,
                        cursor.index as usize + 3 * stream.count as usize,
                    )?);
                    push(&mut self.midi, cursor.last[0])?;
                    cursor.index += 1;
                    update(self.song, stream, cursor, self.time)?;
                }
                consider(*cursor, stream.count, &mut self.next_time);
            }
            if due(state.pitch, base.pitch, self.time) {
                status(&mut self.midi, &mut last_status, 0xe0 | channel as u8)?;
                // Keep the original Tick's program-count pitch-data offsets.
                for component in 0..2 {
                    let value = byte(
                        self.song,
                        base.pitch,
                        state.pitch.index as usize + (3 + component) * base.programs.count as usize,
                    )?;
                    state.pitch.last[component] = state.pitch.last[component].wrapping_add(value);
                    push(&mut self.midi, state.pitch.last[component])?;
                }
                state.pitch.index += 1;
                update(self.song, base.pitch, &mut state.pitch, self.time)?;
            }
            consider(state.pitch, base.pitch.count, &mut self.next_time);
            while due(state.notes, base.notes, self.time) {
                status(&mut self.midi, &mut last_status, 0x90 | channel as u8)?;
                for component in 0..2 {
                    let value = byte(
                        self.song,
                        base.notes,
                        state.notes.index as usize + (3 + component) * base.notes.count as usize,
                    )?;
                    state.notes.last[component] = state.notes.last[component].wrapping_add(value);
                    push(&mut self.midi, state.notes.last[component])?;
                }
                state.notes.index += 1;
                update(self.song, base.notes, &mut state.notes, self.time)?;
            }
            consider(state.notes, base.notes.count, &mut self.next_time);
        }
        push(&mut self.midi, 0xfd)?;
        if self.next_time == u32::MAX {
            self.playing = false;
        }
        Ok(())
    }
}
fn byte(song: &Song<'_>, stream: Stream<'_>, index: usize) -> Result<u8, Error> {
    let offset = (stream.data.as_ptr() as usize)
        .checked_sub(song.data.as_ptr() as usize)
        .and_then(|n| n.checked_add(index))
        .ok_or(Error("V2M stream address overflow"))?;
    song.data
        .get(offset)
        .copied()
        .ok_or(Error("V2M stream read outside song"))
}
fn delta(song: &Song<'_>, stream: Stream<'_>, index: u32, stride: u32) -> Result<u32, Error> {
    let index = index as usize;
    let stride = stride as usize;
    Ok(u32::from(byte(song, stream, index)?)
        | (u32::from(byte(song, stream, index + stride)?) << 8)
        | (u32::from(byte(song, stream, index + 2 * stride)?) << 16))
}
fn initial(
    song: &Song<'_>,
    stream: Stream<'_>,
    stride: u32,
    cursor: &mut Cursor,
    next: &mut u32,
) -> Result<(), Error> {
    if stride > 0 {
        cursor.next_time = delta(song, stream, 0, stride)?;
        *next = (*next).min(cursor.next_time);
    }
    Ok(())
}
fn update(
    song: &Song<'_>,
    stream: Stream<'_>,
    cursor: &mut Cursor,
    time: u32,
) -> Result<(), Error> {
    if cursor.index < stream.count {
        let d = delta(song, stream, cursor.index, stream.count)?;
        if d != 0 {
            cursor.next_time = time.wrapping_add(d);
        }
    }
    Ok(())
}
fn consider(cursor: Cursor, count: u32, next: &mut u32) {
    if cursor.index < count {
        *next = (*next).min(cursor.next_time);
    }
}
fn due(cursor: Cursor, stream: Stream<'_>, time: u32) -> bool {
    cursor.index < stream.count && cursor.next_time == time
}
fn push(midi: &mut Vec<u8>, value: u8) -> Result<(), Error> {
    if midi.len() == 4096 {
        return Err(Error("V2M MIDI buffer capacity exceeded"));
    }
    midi.push(value);
    Ok(())
}
fn status(midi: &mut Vec<u8>, last: &mut Option<u8>, value: u8) -> Result<(), Error> {
    if *last != Some(value) {
        push(midi, value)?;
        *last = Some(value);
    }
    Ok(())
}

/// Original x86 UpdateSampleDelta: the integer division remainder accumulates
/// in a wrapping 32-bit word, with its carry added to the sample count.
pub fn sample_delta(
    next_time: u32,
    time: u32,
    microseconds: u32,
    division: u32,
    remainder: &mut u32,
) -> Result<u32, Error> {
    if division == 0 {
        return Err(Error("zero V2M sample division"));
    }
    let product = u64::from(next_time.wrapping_sub(time)) * u64::from(microseconds);
    let quotient = product / u64::from(division);
    let quotient = u32::try_from(quotient).map_err(|_| Error("V2M sample division overflow"))?;
    let (sum, carry) = remainder.overflowing_add((product % u64::from(division)) as u32);
    *remainder = sum;
    Ok(quotient.wrapping_add(u32::from(carry)))
}
