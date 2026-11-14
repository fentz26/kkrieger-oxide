//! Bounds-checked reader for Werkkzeug's exported player graph.
//!
//! Ported from Farbrausch's KDoc::Init and compact value decoders; see
//! third_party/farbrausch-LICENSE.txt. The legacy header/one-byte class IDs
//! are retained because the published kkrieger3383.kx predates its loader.
use std::{collections::BTreeMap, fmt};

const FLEX_INPUT: u32 = 0x8000_0000;
const BLOB: u32 = 0x0008_0000;
const DONT_CALL_LINK: u32 = 0x1000_0000;
const MAX_FILE: usize = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    /// Published development data: song, samples, timing, 16 roots, u8 class IDs.
    Legacy3383,
    /// Later source loader: flags, optional title/samples, timing, u16 class IDs.
    Current,
    /// 2004 export: no timing/root table; Text has 16 extra bytes.
    Beta2004,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub offset: usize,
    pub reason: &'static str,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "KX byte 0x{:x}: {}", self.offset, self.reason)
    }
}
impl std::error::Error for Error {}
type Result<T> = std::result::Result<T, Error>;

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}
impl<'a> Reader<'a> {
    fn fail<T>(&self, reason: &'static str) -> Result<T> {
        Err(Error {
            offset: self.pos,
            reason,
        })
    }
    fn take(&mut self, count: usize) -> Result<&'a [u8]> {
        let end = self.pos.checked_add(count).ok_or(Error {
            offset: self.pos,
            reason: "length overflow",
        })?;
        let slice = self.bytes.get(self.pos..end).ok_or(Error {
            offset: self.pos,
            reason: "truncated data",
        })?;
        self.pos = end;
        Ok(slice)
    }
    fn byte(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn short(&mut self) -> Result<usize> {
        let v = self.byte()?;
        Ok(if v & 128 != 0 {
            (v as usize & 127) | ((self.byte()? as usize) << 7)
        } else {
            v as usize
        })
    }
    fn cstr(&mut self) -> Result<&'a [u8]> {
        let size = self.bytes[self.pos..]
            .iter()
            .position(|&v| v == 0)
            .ok_or(Error {
                offset: self.pos,
                reason: "unterminated string",
            })?;
        let s = self.take(size)?;
        self.take(1)?;
        Ok(s)
    }
    fn aligned_block(&mut self) -> Result<&'a [u8]> {
        let size = self.u32()? as usize;
        let b = self.take(size)?;
        self.take((4 - size % 4) % 4)?;
        Ok(b)
    }
    fn f16(&mut self) -> Result<f32> {
        let v = self.byte()? as u32;
        Ok(match v {
            0 => 0.,
            0x80 => 1.,
            1 => 0.5,
            0x81 => 0.25,
            _ => {
                let v = (v << 8) | self.byte()? as u32;
                f32::from_bits(
                    ((v & 32768) << 16) | ((((v >> 10) & 31) + 112) << 23) | ((v & 1023) << 13),
                )
            }
        })
    }
    fn f24(&mut self) -> Result<f32> {
        let first = self.byte()? as u32;
        Ok(match first {
            0 => 0.,
            1 => 1.,
            255 => -1.,
            _ => {
                let v = self.u16()? as u32;
                f32::from_bits((first << 23) | ((v & 32768) << 16) | ((v & 32767) << 8))
            }
        })
    }
    fn reference(&mut self, count: usize) -> Result<Option<usize>> {
        let value = self.short()?;
        if value > count {
            return self.fail("reference out of range");
        }
        Ok(value.checked_sub(1))
    }
    fn animation(&mut self) -> Result<&'a [u8]> {
        let start = self.pos;
        loop {
            let command = self.byte()?;
            let payload = match command {
                0..=1 | 7..=21 | 26..=28 => 0,
                2..=6 => 1,
                22 => 16,
                23..=24 => 4,
                25 => 2,
                29 => 8,
                0x80..=0xbf | 0xd0..=0xff => 1,
                _ => return self.fail("unknown animation opcode"),
            };
            self.take(payload)?;
            if command == 1 {
                return Ok(&self.bytes[start..self.pos]);
            }
        }
    }
}

#[derive(Debug)]
pub struct Class<'a> {
    pub id: u16,
    pub convention: u32,
    pub packing: &'a [u8],
}
impl Class<'_> {
    pub fn data_words(&self) -> usize {
        (self.convention & 255) as usize
    }
    pub fn link_count(&self) -> usize {
        ((self.convention >> 12) & 15) as usize
    }
    pub fn string_count(&self) -> usize {
        ((self.convention >> 16) & 7) as usize
    }
    pub fn spline_count(&self) -> usize {
        ((self.convention >> 20) & 7) as usize
    }
}

#[derive(Debug)]
pub struct Operator<'a> {
    pub class: usize,
    pub inputs: Vec<usize>,
    pub links: Vec<Option<usize>>,
    /// Decoded 32-bit words, preserving float and signed-integer bit patterns.
    pub parameters: Vec<u32>,
    pub packed_parameters: &'a [u8],
    /// Original strings are byte strings (Windows ANSI), not assumed UTF-8.
    pub strings: Vec<&'a [u8]>,
    pub splines: Vec<Option<usize>>,
    pub animation: &'a [u8],
    pub blob: &'a [u8],
    blob_size: usize,
}
impl Operator<'_> {
    pub fn float(&self, index: usize) -> Option<f32> {
        self.parameters.get(index).map(|&p| f32::from_bits(p))
    }
    pub fn integer(&self, index: usize) -> Option<i32> {
        self.parameters.get(index).map(|&p| p as i32)
    }
}

#[derive(Debug)]
pub struct Event {
    pub operator: usize,
    pub start: i32,
    pub end: i32,
    pub velocity: f32,
    pub modulation: f32,
    pub select: u8,
    pub scale: [f32; 3],
    pub rotate: [f32; 3],
    pub translate: [f32; 3],
    pub color: u32,
    pub spline: Option<usize>,
    pub interval: [f32; 2],
}
#[derive(Debug)]
pub struct Key {
    pub time: f32,
    pub value: f32,
}
#[derive(Debug)]
pub struct Spline {
    pub interpolation: u8,
    pub channels: Vec<Vec<Key>>,
}

#[derive(Debug)]
pub struct Document<'a> {
    pub layout: Layout,
    pub flags: u32,
    pub title: Option<&'a [u8]>,
    pub song: &'a [u8],
    pub samples: &'a [u8],
    pub bpm: u32,
    pub song_length: u32,
    pub roots: [Option<usize>; 16],
    pub classes: Vec<Class<'a>>,
    pub operators: Vec<Operator<'a>>,
    pub events: Vec<Event>,
    pub splines: Vec<Spline>,
}
impl<'a> Document<'a> {
    /// Detect current vs. development header. The headerless 2004 format must be
    /// selected explicitly: it cannot be distinguished reliably from legacy data.
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        let first = bytes.get(..4).ok_or(Error {
            offset: 0,
            reason: "missing header",
        })?;
        let layout = if u32::from_le_bytes(first.try_into().unwrap()) & !7 == 0 {
            Layout::Current
        } else {
            Layout::Legacy3383
        };
        Self::parse_layout(bytes, layout)
    }
    pub fn parse_layout(bytes: &'a [u8], layout: Layout) -> Result<Self> {
        let mut r = Reader { bytes, pos: 0 };
        if bytes.len() > MAX_FILE {
            return r.fail("file exceeds 16 MiB limit");
        }
        let flags = if layout == Layout::Current {
            r.u32()?
        } else {
            2
        };
        if flags & !7 != 0 {
            return r.fail("unknown header flags");
        }
        let title = if flags & 1 != 0 {
            Some(r.take(32)?)
        } else {
            None
        };
        let song = r.aligned_block()?;
        let samples = if flags & 2 != 0 {
            r.aligned_block()?
        } else {
            &[]
        };
        let (bpm, song_length) = if layout == Layout::Beta2004 {
            (0, 0)
        } else {
            (r.u32()?, r.u32()?)
        };
        let n_ops = r.short()?;
        let n_splines = r.short()?;
        if n_ops == 0 {
            return r.fail("empty operator graph");
        }
        let mut roots = [None; 16];
        if layout == Layout::Beta2004 {
            roots[0] = Some(n_ops - 1);
        } else {
            for root in &mut roots {
                let index = r.short()?;
                *root = (index < n_ops).then_some(index);
            }
        }
        let mut classes = Vec::new();
        loop {
            let convention = r.u32()?;
            if convention == 0 {
                break;
            }
            if classes.len() == 128 {
                return r.fail("class table exceeds 7-bit index space");
            }
            let id = if layout == Layout::Current {
                r.u16()?
            } else {
                r.byte()? as u16
            };
            let packing = r.cstr()?;
            // The shipped 3383 Multiply schema declares 17 words but packs 18.
            // Preserve every encoded word rather than repeating the C loader's
            // out-of-bounds write or discarding the eighteenth parameter.
            if packing.len() > 256 {
                return r.fail("packing exceeds parameter limit");
            }
            if classes.iter().any(|c: &Class<'_>| c.id == id) {
                return r.fail("duplicate operator class");
            }
            classes.push(Class {
                id,
                convention,
                packing,
            });
        }
        let mut operators = Vec::with_capacity(n_ops);
        for i in 0..n_ops {
            let ty = r.byte()?;
            let class = (ty & 127) as usize;
            let cls = classes.get(class).ok_or(Error {
                offset: r.pos - 1,
                reason: "operator class out of range",
            })?;
            let shorthand = ty & 128 != 0;
            let input_count = if shorthand {
                1
            } else if cls.convention & FLEX_INPUT != 0 {
                r.byte()? as usize
            } else {
                ((cls.convention >> 8) & 15) as usize
            };
            let mut inputs = Vec::with_capacity(input_count);
            for _ in 0..input_count {
                let delta = if shorthand { 0 } else { r.short()? };
                inputs.push(i.checked_sub(delta + 1).ok_or(Error {
                    offset: r.pos,
                    reason: "input is not an earlier operator",
                })?);
            }
            operators.push(Operator {
                class,
                inputs,
                links: Vec::new(),
                parameters: vec![0; cls.data_words().max(cls.packing.len())],
                packed_parameters: &[],
                strings: Vec::new(),
                splines: Vec::new(),
                animation: &[],
                blob: &[],
                blob_size: 0,
            });
        }
        for op in &mut operators {
            for _ in 0..classes[op.class].link_count() {
                op.links.push(r.reference(n_ops)?);
            }
        }
        // The export groups parameter blocks by class, not by operator index.
        for (ci, cls) in classes.iter().enumerate() {
            for op in operators.iter_mut().filter(|o| o.class == ci) {
                let record_start = r.pos;
                for (word, &format) in op.parameters.iter_mut().zip(cls.packing) {
                    *word = match format.to_ascii_lowercase() {
                        b'g' => r.f16()?.to_bits(),
                        b'f' => r.f24()?.to_bits(),
                        b'e' => ((r.u16()? as i16) as f32 / 4096.).to_bits(),
                        b'i' | b'c' => r.u32()?,
                        b's' => (r.u16()? as i16 as i32) as u32,
                        b'b' => r.byte()? as u32,
                        b'-' => 0, // omitted/padded parameter, no bytes in the export
                        b'm' => {
                            let b = r.take(3)?;
                            u32::from_le_bytes([b[0], b[1], b[2], 0])
                        }
                        _ => return r.fail("unknown parameter packing"),
                    };
                }
                for _ in 0..cls.string_count() {
                    op.strings.push(r.cstr()?);
                }
                if layout == Layout::Beta2004 && cls.id == 0x31 {
                    r.take(16)?;
                }
                for _ in 0..cls.spline_count() {
                    let index = r.u16()? as usize;
                    if index > n_splines {
                        return r.fail("spline reference out of range");
                    }
                    op.splines.push(index.checked_sub(1));
                }
                if cls.convention & BLOB != 0 {
                    op.blob_size = r.u32()? as usize;
                }
                op.packed_parameters = &bytes[record_start..r.pos];
            }
        }
        for op in &mut operators {
            op.animation = r.animation()?;
        }
        let n_events = r.short()?;
        let mut events = Vec::with_capacity(n_events);
        for _ in 0..n_events {
            let operator = r.short()?;
            if operator >= n_ops {
                return r.fail("event operator out of range");
            }
            let start = r.u32()? as i32;
            let end = r.u32()? as i32;
            let velocity = r.f24()?;
            let modulation = r.f24()?;
            let select = r.byte()?;
            let scale = [r.f24()?, r.f24()?, r.f24()?];
            let rotate = [r.f24()?, r.f24()?, r.f24()?];
            let translate = [r.f24()?, r.f24()?, r.f24()?];
            let color = r.u32()?;
            let spline = r.reference(n_splines)?;
            let interval = if layout == Layout::Beta2004 {
                [0., 1.]
            } else {
                [r.f16()?, r.f16()?]
            };
            events.push(Event {
                operator,
                start,
                end,
                velocity,
                modulation,
                select,
                scale,
                rotate,
                translate,
                color,
                spline,
                interval,
            });
        }
        let mut splines = Vec::with_capacity(n_splines);
        for _ in 0..n_splines {
            let header = r.byte()?;
            let count = header & 7;
            let interpolation = header >> 3;
            if count > 4 || interpolation > 2 {
                return r.fail("invalid spline format");
            }
            let mut channels = Vec::with_capacity(count as usize);
            for _ in 0..count {
                let keys = r.short()?;
                let mut channel = Vec::with_capacity(keys);
                for _ in 0..keys {
                    channel.push(Key {
                        time: r.byte()? as f32 / 255.,
                        value: r.f16()?,
                    });
                }
                if channel.windows(2).any(|k| k[0].time > k[1].time) {
                    return r.fail("unsorted spline keys");
                }
                channels.push(channel);
            }
            splines.push(Spline {
                interpolation,
                channels,
            });
        }
        for op in &mut operators {
            op.blob = r.take(op.blob_size)?;
        }
        if r.pos != bytes.len() {
            return r.fail("trailing bytes after export");
        }
        Ok(Self {
            layout,
            flags,
            title,
            song,
            samples,
            bpm,
            song_length,
            roots,
            classes,
            operators,
            events,
            splines,
        })
    }

    pub fn histogram(&self) -> BTreeMap<u16, usize> {
        let mut counts = BTreeMap::new();
        for op in &self.operators {
            *counts.entry(self.classes[op.class].id).or_default() += 1;
        }
        counts
    }
    /// Includes event links: unlike initialization, a full port must implement
    /// event-only assets too. Visited bits make cyclic event references safe.
    pub fn reachable(&self, root: usize) -> Result<Vec<usize>> {
        if root >= self.operators.len() {
            return Err(Error {
                offset: 0,
                reason: "root out of range",
            });
        }
        let mut visited = vec![false; self.operators.len()];
        let mut pending = vec![root];
        while let Some(i) = pending.pop() {
            if visited[i] {
                continue;
            }
            visited[i] = true;
            let op = &self.operators[i];
            pending.extend(op.inputs.iter().copied());
            pending.extend(op.links.iter().flatten().copied());
        }
        Ok(visited
            .iter()
            .enumerate()
            .filter_map(|(i, &v)| v.then_some(i))
            .collect())
    }
    /// Dependencies before users, respecting OPC_DONTCALLLINK. Iterative DFS
    /// avoids consuming the call stack on long procedural chains.
    pub fn initialization_order(&self, root: usize) -> Result<Vec<usize>> {
        if root >= self.operators.len() {
            return Err(Error {
                offset: 0,
                reason: "root out of range",
            });
        }
        let mut colors = vec![0_u8; self.operators.len()];
        let mut stack = vec![(root, false)];
        let mut order = Vec::new();
        while let Some((i, exit)) = stack.pop() {
            if exit {
                colors[i] = 2;
                order.push(i);
                continue;
            }
            match colors[i] {
                2 => continue,
                1 => {
                    return Err(Error {
                        offset: i,
                        reason: "cycle in initialization dependencies",
                    });
                }
                _ => {}
            }
            colors[i] = 1;
            stack.push((i, true));
            let op = &self.operators[i];
            if self.classes[op.class].convention & DONT_CALL_LINK == 0 {
                stack.extend(op.links.iter().flatten().rev().map(|&i| (i, false)));
            }
            stack.extend(op.inputs.iter().rev().map(|&i| (i, false)));
        }
        Ok(order)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compact_values_follow_original_special_cases() {
        let mut r = Reader {
            bytes: &[
                0, 0x80, 1, 0x81, 0x40, 0, 0xc0, 0, 0, 1, 255, 128, 0, 0, 0xff, 0xff,
            ],
            pos: 0,
        };
        assert_eq!(
            [
                r.f16().unwrap(),
                r.f16().unwrap(),
                r.f16().unwrap(),
                r.f16().unwrap()
            ],
            [0., 1., 0.5, 0.25]
        );
        assert_eq!(r.f16().unwrap(), 2.);
        assert_eq!(r.f16().unwrap(), -2.);
        assert_eq!(
            [r.f24().unwrap(), r.f24().unwrap(), r.f24().unwrap()],
            [0., 1., -1.]
        );
        assert_eq!(r.f24().unwrap(), 2.);
        assert_eq!(r.short().unwrap(), 32767);
    }
}
