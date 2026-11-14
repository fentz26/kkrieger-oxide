//! Small deterministic synthesizer. No samples, codecs, or audio allocations in the callback.
use std::f32::consts::TAU;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub struct Synth {
    frame: u64,
    phase: [f32; 3],
    noise: u32,
    shot: f32,
}
impl Synth {
    pub fn new() -> Self {
        Self {
            frame: 0,
            phase: [0.; 3],
            noise: 0x1234abcd,
            shot: 0.,
        }
    }
    pub fn trigger(&mut self) {
        self.shot = 1.;
    }
    pub fn render(&mut self, output: &mut [f32]) {
        for stereo in output.as_chunks_mut::<2>().0 {
            let t = self.frame as f32 / 44100.;
            let step = ((self.frame / 11025) % 16) as usize;
            let note = [
                0., 0., 7., 0., 3., 0., 10., 7., 0., 0., 7., 12., 3., 7., 10., 7.,
            ][step];
            let bass = 55. * 2_f32.powf(note / 12.);
            for (p, f) in self.phase.iter_mut().zip([bass, 110.03, 164.81]) {
                *p = (*p + f / 44100.).fract();
            }
            self.noise ^= self.noise << 13;
            self.noise ^= self.noise >> 17;
            self.noise ^= self.noise << 5;
            let noise = self.noise as f32 / u32::MAX as f32 * 2. - 1.;
            let beat = (self.frame % 11025) as f32 / 11025.;
            let pulse = (1. - beat).powi(8);
            let drone = (self.phase[1] * TAU).sin() * 0.025 + (self.phase[2] * TAU).sin() * 0.018;
            let seq = (self.phase[0] * TAU).sin() * pulse * 0.075;
            let kick = (TAU * (48. * beat + 3. * (1. - (-beat * 20.).exp()))).sin() * pulse * 0.055;
            let shot = (noise * 0.5 + (t * TAU * (180. + self.shot * 400.)).sin() * 0.5)
                * self.shot
                * 0.25;
            self.shot *= 0.9992;
            let s = drone + seq + kick + shot;
            let pan = (t * 0.3).sin() * 0.15;
            stereo[0] = s * (1. - pan);
            stereo[1] = s * (1. + pan);
            self.frame += 1;
        }
    }
}

pub struct Audio {
    pub fire: Arc<AtomicBool>,
    pub muted: Arc<AtomicBool>,
    #[cfg(target_os = "macos")]
    native: Option<native::Output>,
}
impl Audio {
    pub fn new() -> Self {
        let fire = Arc::new(AtomicBool::new(false));
        let muted = Arc::new(AtomicBool::new(false));
        #[cfg(target_os = "macos")]
        let native = native::Output::new(fire.clone(), muted.clone());
        Self {
            fire,
            muted,
            #[cfg(target_os = "macos")]
            native,
        }
    }
    pub fn available(&self) -> bool {
        #[cfg(target_os = "macos")]
        {
            self.native.is_some()
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }
    pub fn shoot(&self) {
        self.fire.store(true, Ordering::Relaxed);
    }
    pub fn toggle_mute(&self) {
        self.muted.fetch_xor(true, Ordering::Relaxed);
    }
}

#[cfg(target_os = "macos")]
mod native {
    use super::*;
    use std::ffi::c_void;
    use std::ptr::{null, null_mut};

    #[repr(C)]
    struct Format {
        rate: f64,
        id: u32,
        flags: u32,
        packet_bytes: u32,
        packet_frames: u32,
        frame_bytes: u32,
        channels: u32,
        bits: u32,
        reserved: u32,
    }
    #[repr(C)]
    struct Buffer {
        capacity: u32,
        data: *mut c_void,
        size: u32,
        user: *mut c_void,
        packet_capacity: u32,
        packets: *mut c_void,
        packet_count: u32,
    }
    type Queue = *mut c_void;
    #[link(name = "AudioToolbox", kind = "framework")]
    unsafe extern "C" {
        fn AudioQueueNewOutput(
            f: *const Format,
            cb: unsafe extern "C" fn(*mut c_void, Queue, *mut Buffer),
            user: *mut c_void,
            run_loop: *const c_void,
            mode: *const c_void,
            flags: u32,
            out: *mut Queue,
        ) -> i32;
        fn AudioQueueAllocateBuffer(q: Queue, size: u32, out: *mut *mut Buffer) -> i32;
        fn AudioQueueEnqueueBuffer(
            q: Queue,
            b: *mut Buffer,
            count: u32,
            desc: *const c_void,
        ) -> i32;
        fn AudioQueueStart(q: Queue, time: *const c_void) -> i32;
        fn AudioQueueDispose(q: Queue, immediate: u8) -> i32;
    }
    struct State {
        synth: Synth,
        fire: Arc<AtomicBool>,
        muted: Arc<AtomicBool>,
    }
    pub struct Output {
        queue: Queue,
        state: *mut State,
    }

    unsafe fn fill(state: *mut State, b: *mut Buffer) {
        // SAFETY: State is exclusively callback-owned after AudioQueueStart. Buffers are
        // allocated by AudioQueue, float-aligned, and capacity is a multiple of a stereo frame.
        unsafe {
            let state = &mut *state;
            let samples =
                std::slice::from_raw_parts_mut((*b).data.cast::<f32>(), (*b).capacity as usize / 4);
            if state.fire.swap(false, Ordering::Relaxed) {
                state.synth.trigger();
            }
            state.synth.render(samples);
            if state.muted.load(Ordering::Relaxed) {
                samples.fill(0.);
            }
            (*b).size = (*b).capacity;
        }
    }
    unsafe extern "C" fn callback(user: *mut c_void, q: Queue, b: *mut Buffer) {
        // SAFETY: AudioQueue supplies the registered state and a completed writable buffer.
        unsafe {
            fill(user.cast(), b);
            AudioQueueEnqueueBuffer(q, b, 0, null());
        }
    }
    impl Output {
        pub fn new(fire: Arc<AtomicBool>, muted: Arc<AtomicBool>) -> Option<Self> {
            let state = Box::into_raw(Box::new(State {
                synth: Synth::new(),
                fire,
                muted,
            }));
            let format = Format {
                rate: 44100.,
                id: u32::from_be_bytes(*b"lpcm"),
                flags: 1 | 8,
                packet_bytes: 8,
                packet_frames: 1,
                frame_bytes: 8,
                channels: 2,
                bits: 32,
                reserved: 0,
            };
            let mut queue = null_mut();
            // SAFETY: repr(C) layouts and signatures match the macOS AudioToolbox SDK.
            // The heap state remains pinned until synchronous disposal stops all callbacks.
            unsafe {
                if AudioQueueNewOutput(
                    &format,
                    callback,
                    state.cast(),
                    null(),
                    null(),
                    0,
                    &mut queue,
                ) != 0
                {
                    drop(Box::from_raw(state));
                    eprintln!("Audio output unavailable; continuing silently.");
                    return None;
                }
                let output = Self { queue, state };
                for _ in 0..3 {
                    let mut buffer = null_mut();
                    if AudioQueueAllocateBuffer(queue, 1024 * 8, &mut buffer) != 0 {
                        return None;
                    }
                    fill(state, buffer);
                    if AudioQueueEnqueueBuffer(queue, buffer, 0, null()) != 0 {
                        return None;
                    }
                }
                if AudioQueueStart(queue, null()) != 0 {
                    return None;
                }
                Some(output)
            }
        }
    }
    impl Drop for Output {
        fn drop(&mut self) {
            // SAFETY: immediate disposal waits for the audio thread before State is freed.
            unsafe {
                if AudioQueueDispose(self.queue, 1) == 0 {
                    drop(Box::from_raw(self.state));
                }
                // On disposal failure leak State rather than risking a callback use-after-free.
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn synthesis_is_finite_bounded_and_chunk_invariant() {
        let mut whole = vec![0.; 44100 * 2];
        let mut split = vec![0.; 44100 * 2];
        Synth::new().render(&mut whole);
        let mut s = Synth::new();
        for block in split.chunks_mut(882) {
            s.render(block);
        }
        assert_eq!(whole, split);
        assert!(whole.iter().all(|x| x.is_finite() && x.abs() < 1.));
        assert!(whole.iter().any(|x| x.abs() > 0.05));
        s.trigger();
        s.render(&mut split);
        assert!(split.iter().all(|x| x.is_finite() && x.abs() < 1.));
    }
}
