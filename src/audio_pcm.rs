//! Device-independent PCM blocks for the original stereo 44,100 Hz output.
pub(crate) const FRAMES: usize = 1024;
pub(crate) const BYTES: u32 = (FRAMES * size_of::<[i16; 2]>()) as u32;

// Preserve already-quantized music exactly; saturate the combined music/effect
// signal to the original signed 16-bit device range. NaN casts to silence.
pub(crate) fn pcm16(sample: f32) -> i16 {
    ((sample * 32768.0) as i32).clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pcm16_conversion_clips_mixed_audio_and_preserves_original_music_pcm() {
        for value in i16::MIN..=i16::MAX {
            assert_eq!(pcm16(f32::from(value) / 32768.0), value);
        }
        for (value, expected) in [
            (2.0, 32767),
            (-2.0, -32768),
            (1.0, 32767),
            (-1.0, -32768),
            (0.5, 16384),
            (-0.5, -16384),
            (f32::NAN, 0),
            (f32::INFINITY, 32767),
            (f32::NEG_INFINITY, -32768),
        ] {
            assert_eq!(pcm16(value), expected);
        }
        assert_eq!(BYTES, 4096);
    }
}
