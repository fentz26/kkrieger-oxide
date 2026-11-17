//! Original V2M version detection and legacy parameter/modulation remapping.
use super::{Error, Song};
mod tables {
    include!("conversion_tables.rs");
}
use tables::{GLOBAL_DEFAULTS, GLOBAL_VERSIONS, PARAMETER_DEFAULTS, PARAMETER_VERSIONS};
const VERSION: u8 = 6;
fn parameter_count(version: u8) -> usize {
    PARAMETER_VERSIONS.iter().filter(|&&v| v <= version).count()
}
fn offset(data: &[u8], index: usize) -> Result<usize, Error> {
    let bytes = data
        .get(index * 4..index * 4 + 4)
        .ok_or(Error("truncated V2 patch offset table"))?;
    Ok(u32::from_le_bytes(bytes.try_into().unwrap()) as usize)
}
fn detect(song: &Song<'_>) -> Result<u8, Error> {
    if song.patches.is_empty() {
        return Err(Error("V2M contains no patches"));
    }
    // The original checks the stored program bytes, before MIDI delta decoding.
    let used = song
        .channels
        .iter()
        .flat_map(|c| {
            c.programs
                .data
                .get(c.programs.count as usize * 3..)
                .unwrap_or(&[])
                .iter()
                .copied()
        })
        .filter(|&v| v < 128)
        .max()
        .map_or(0, |v| usize::from(v) + 1);
    if used == 0 {
        return Err(Error("V2M contains no program changes"));
    }
    let mut best = None;
    for version in 0..=VERSION {
        if song.globals.len() != GLOBAL_VERSIONS.iter().filter(|&&v| v <= version).count() {
            continue;
        }
        let header = parameter_count(version) + 1;
        let mut matches = true;
        for p in 0..used - 1 {
            let start = offset(song.patches, p)?;
            let end = offset(song.patches, p + 1)?;
            let Some(mod_size) = end
                .checked_sub(start)
                .and_then(|size| size.checked_sub(header))
            else {
                matches = false;
                break;
            };
            if !mod_size.is_multiple_of(3)
                || song
                    .patches
                    .get(start + header - 1)
                    .map(|&n| usize::from(n))
                    != Some(mod_size / 3)
            {
                matches = false;
                break;
            }
        }
        if matches {
            best = Some(version);
        }
    }
    best.ok_or(Error("unknown V2M synth version"))
}
pub fn version_delta(data: &[u8]) -> Result<u8, Error> {
    let song = Song::parse(data)?;
    Ok(VERSION - detect(&song)?)
}
fn word(output: &mut Vec<u8>, value: usize) -> Result<(), Error> {
    let value = u32::try_from(value).map_err(|_| Error("converted V2M size overflow"))?;
    output.extend(value.to_le_bytes());
    Ok(())
}
pub fn convert(data: &[u8]) -> Result<Vec<u8>, Error> {
    let song = Song::parse(data)?;
    let version = detect(&song)?;
    if version == VERSION {
        let mut output = Vec::with_capacity(data.len() + 4);
        output.extend_from_slice(data);
        output.extend([0; 4]);
        return Ok(output);
    }
    let table_size = offset(song.patches, 0)?;
    if table_size == 0
        || !table_size.is_multiple_of(4)
        || table_size > 512
        || table_size > song.patches.len()
    {
        return Err(Error("invalid legacy V2 patch map"));
    }
    let programs = table_size / 4;
    let old_parameters = parameter_count(version);
    let parameter_difference = PARAMETER_DEFAULTS.len() - old_parameters;
    let global_difference = GLOBAL_DEFAULTS.len() - song.globals.len();
    let new_size = data
        .len()
        .checked_add(global_difference + programs * parameter_difference + 4)
        .ok_or(Error("converted V2M size overflow"))?;
    let midi_size = song.globals.as_ptr() as usize - data.as_ptr() as usize - 4;
    let mut output = Vec::with_capacity(new_size);
    output.extend_from_slice(&data[..midi_size]);
    word(&mut output, GLOBAL_DEFAULTS.len())?;
    let mut globals = GLOBAL_DEFAULTS;
    let mut old_globals = song.globals.iter();
    for (value, &introduced) in globals.iter_mut().zip(&GLOBAL_VERSIONS) {
        if introduced <= version {
            *value = *old_globals
                .next()
                .ok_or(Error("truncated legacy V2 globals"))?;
        }
    }
    output.extend(globals);
    let patch_size = song.patches.len() + programs * parameter_difference;
    word(&mut output, patch_size)?;
    let patch_start = output.len();
    for program in 0..programs {
        word(
            &mut output,
            offset(song.patches, program)? + program * parameter_difference,
        )?;
    }
    for program in 0..programs {
        let start = offset(song.patches, program)?;
        let old = song
            .patches
            .get(start..)
            .ok_or(Error("legacy patch outside bank"))?;
        let mut source = old.iter();
        let mut parameters = PARAMETER_DEFAULTS;
        for (value, &introduced) in parameters.iter_mut().zip(&PARAMETER_VERSIONS) {
            if introduced <= version {
                *value = *source
                    .next()
                    .ok_or(Error("truncated legacy V2 parameters"))?;
            }
        }
        output.extend(parameters);
        let count = *source
            .next()
            .ok_or(Error("missing legacy V2 modulation count"))?;
        output.push(count);
        for _ in 0..count {
            let source_index = *source
                .next()
                .ok_or(Error("truncated legacy V2 modulation source"))?;
            let amount = *source
                .next()
                .ok_or(Error("truncated legacy V2 modulation amount"))?;
            let mut destination = usize::from(
                *source
                    .next()
                    .ok_or(Error("truncated legacy V2 modulation destination"))?,
            );
            let mut index = 0;
            while index <= destination {
                if *PARAMETER_VERSIONS
                    .get(index)
                    .ok_or(Error("invalid legacy V2 modulation destination"))?
                    > version
                {
                    destination += 1;
                }
                index += 1;
            }
            output.extend([source_index, amount, destination as u8]);
        }
    }
    if output.len() != patch_start + patch_size {
        return Err(Error("inconsistent legacy V2 patch sizes"));
    }
    let speech_start = song.patches.as_ptr() as usize - data.as_ptr() as usize + song.patches.len();
    let speech_size = offset(
        data.get(speech_start..)
            .ok_or(Error("missing V2M speech block"))?,
        0,
    )?;
    if speech_size > 8192 {
        word(&mut output, 0)?;
    } else {
        word(&mut output, speech_size)?;
        output.extend_from_slice(
            data.get(speech_start + 4..speech_start + 4 + speech_size)
                .ok_or(Error("truncated V2M speech block"))?,
        );
    }
    if output.len() > new_size {
        return Err(Error("converted V2M exceeds expected size"));
    }
    output.resize(new_size, 0);
    Ok(output)
}
