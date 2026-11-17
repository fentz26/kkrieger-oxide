//! Original V2M InitBase layout, borrowed directly from the embedded song.
#[derive(Clone, Copy, Default)]
pub struct Stream<'a> {
    pub count: u32,
    pub data: &'a [u8],
}
#[derive(Clone, Copy, Default)]
pub struct Channel<'a> {
    pub notes: Stream<'a>,
    pub programs: Stream<'a>,
    pub pitch: Stream<'a>,
    pub controls: [Stream<'a>; 7],
}
pub struct Song<'a> {
    data: &'a [u8],
    pub time_division: u32,
    pub scaled_time_division: u32,
    pub maximum_time: u32,
    pub global_events: Stream<'a>,
    pub channels: [Channel<'a>; 16],
    pub globals: &'a [u8],
    pub patches: &'a [u8],
    pub speech_data: Option<&'a [u8]>,
    pub speech: [Option<&'a [u8]>; 256],
    pub consumed: usize,
}
#[derive(Debug, PartialEq, Eq)]
pub struct Error(pub &'static str);
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for Error {}
struct Reader<'a> {
    data: &'a [u8],
    position: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, size: usize) -> Result<&'a [u8], Error> {
        let end = self
            .position
            .checked_add(size)
            .ok_or(Error("V2M size overflow"))?;
        let data = self
            .data
            .get(self.position..end)
            .ok_or(Error("truncated V2M data"))?;
        self.position = end;
        Ok(data)
    }
    fn word(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn stream(&mut self, stride: usize) -> Result<Stream<'a>, Error> {
        let count = self.word()?;
        let size = (count as usize)
            .checked_mul(stride)
            .ok_or(Error("V2M event count overflow"))?;
        Ok(Stream {
            count,
            data: self.take(size)?,
        })
    }
    fn block(&mut self, limit: usize) -> Result<&'a [u8], Error> {
        let size = self.word()? as usize;
        if size > limit {
            return Err(Error("V2M block exceeds original size limit"));
        }
        self.take(size)
    }
}
impl<'a> Song<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self, Error> {
        let mut r = Reader { data, position: 0 };
        let time_division = r.word()?;
        let maximum_time = r.word()?;
        let global_events = r.stream(10)?;
        let mut channels = [Channel::default(); 16];
        for channel in &mut channels {
            channel.notes = r.stream(5)?;
            if channel.notes.count != 0 {
                channel.programs = r.stream(4)?;
                channel.pitch = r.stream(5)?;
                for control in &mut channel.controls {
                    *control = r.stream(4)?;
                }
            }
        }
        let globals = r.block(16384)?;
        let patches = r.block(1048576)?;
        let speech_size = r.word()? as usize;
        let mut speech: [Option<&'a [u8]>; 256] = [None; 256];
        let speech_data = if speech_size == 0 || speech_size >= 8192 {
            speech.fill(Some(b" "));
            None
        } else {
            let data = r.take(speech_size)?;
            let mut words = Reader { data, position: 0 };
            let count = words.word()? as usize;
            if count > speech.len() {
                return Err(Error("V2M speech count exceeds 256"));
            }
            for entry in &mut speech[..count] {
                let offset = words.word()? as usize;
                let text = data
                    .get(offset..)
                    .ok_or(Error("V2M speech offset out of range"))?;
                let length = text
                    .iter()
                    .position(|&b| b == 0)
                    .ok_or(Error("unterminated V2M speech string"))?;
                *entry = Some(&text[..length]);
            }
            Some(data)
        };
        Ok(Self {
            data,
            time_division,
            scaled_time_division: time_division.wrapping_mul(10000),
            maximum_time,
            global_events,
            channels,
            globals,
            patches,
            speech_data,
            speech,
            consumed: r.position,
        })
    }
}

pub mod sequence;
