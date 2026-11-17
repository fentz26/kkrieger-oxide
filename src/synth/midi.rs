//! Original ViruZ II MIDI packet parsing and persistent running status.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Message {
    pub status: u8,
    pub data: [u8; 2],
    pub length: u8,
}
#[derive(Clone, Copy, Default)]
pub struct Decoder {
    pub running_status: u8,
}
impl Decoder {
    /// Return the position of the FD terminator (or the original invalid-status
    /// stop). A packet may contain trailing bytes after that position.
    pub fn process(
        &mut self,
        packet: &[u8],
        mut emit: impl FnMut(Message) -> Result<(), &'static str>,
    ) -> Result<usize, &'static str> {
        let mut cursor = 0;
        loop {
            let byte = *packet.get(cursor).ok_or("unterminated V2 MIDI packet")?;
            if byte & 0x80 != 0 {
                if byte == 0xfd {
                    return Ok(cursor);
                }
                self.running_status = byte;
                cursor += 1;
            }
            if self.running_status & 0x80 == 0 {
                return Ok(cursor);
            }
            let command = (self.running_status >> 4) & 7;
            let length = [2, 2, 2, 2, 1, 1, 2, 0][usize::from(command)];
            let input = packet
                .get(cursor..cursor + usize::from(length))
                .ok_or("truncated V2 MIDI message")?;
            let mut data = [0; 2];
            data[..input.len()].copy_from_slice(input);
            let status = self.running_status;
            emit(Message {
                status,
                data,
                length,
            })?;
            cursor += usize::from(length);
            if status == 0xff {
                self.running_status = 0;
            }
            // The original system handler consumes no data. Reject packets that
            // would repeatedly dispatch the same data byte forever.
            if length == 0
                && self.running_status & 0x80 != 0
                && packet.get(cursor).is_some_and(|&byte| byte & 0x80 == 0)
            {
                return Err("V2 MIDI system message does not advance");
            }
        }
    }
}
