//! Nonblocking ALSA output on the owning music worker.
use crate::audio_pcm::{BYTES, FRAMES, pcm16};
const _: () = assert!(size_of::<[[i16; 2]; FRAMES]>() == BYTES as usize);
use std::panic::{AssertUnwindSafe, catch_unwind};

const INTERNAL: i32 = i32::MIN;
const UNAVAILABLE: i32 = i32::MIN + 1;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Error(pub i32);
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            UNAVAILABLE => {
                f.write_str("libasound.so.2 or a required ALSA entry point is unavailable")
            }
            INTERNAL => f.write_str("Linux audio renderer or driver failed"),
            code => write!(f, "Linux ALSA audio error {code}"),
        }
    }
}
impl std::error::Error for Error {}

// Mirror ALSA's prepare/resume policy without its potentially blocking sleep loop.
fn recover_stream(
    code: i32,
    resume: impl FnOnce() -> i32,
    prepare: impl FnOnce() -> i32,
) -> Result<(), Error> {
    if code == -86 {
        match resume() {
            0 => return Ok(()),
            error @ (-11 | -4) => return Err(Error(error)),
            _ => {}
        }
    }
    let result = prepare();
    if result < 0 {
        Err(Error(result))
    } else {
        Ok(())
    }
}

// Writes consume/copy samples synchronously; drivers never retain this slice.
trait Driver {
    fn write(&mut self, samples: &[[i16; 2]]) -> Result<usize, Error>;
    fn recover(&mut self, code: i32) -> Result<(), Error>;
}
struct Engine<F: FnMut(&mut [[f32; 2]]), D: Driver> {
    driver: D,
    render: F,
    scratch: [[f32; 2]; FRAMES],
    samples: [[i16; 2]; FRAMES],
    offset: usize,
    status: i32,
}
impl<F: FnMut(&mut [[f32; 2]]), D: Driver> Engine<F, D> {
    fn open(driver: D, render: F) -> Result<Self, Error> {
        let mut e = Self {
            driver,
            render,
            scratch: [[0.; 2]; FRAMES],
            samples: [[0; 2]; FRAMES],
            offset: FRAMES,
            status: 0,
        };
        e.pump()?;
        Ok(e)
    }
    fn pump(&mut self) -> Result<(), Error> {
        if self.status != 0 {
            return Err(Error(self.status));
        }
        let result = self.transfer();
        if let Err(Error(code)) = result {
            self.status = code;
        }
        result
    }
    fn transfer(&mut self) -> Result<(), Error> {
        // Even an always-writable virtual device cannot monopolize the worker.
        for _ in 0..3 {
            if self.offset == FRAMES {
                self.scratch.fill([0.; 2]);
                catch_unwind(AssertUnwindSafe(|| (self.render)(&mut self.scratch)))
                    .map_err(|_| Error(INTERNAL))?;
                for (out, input) in self.samples.iter_mut().zip(&self.scratch) {
                    *out = input.map(pcm16);
                }
                self.offset = 0;
            }
            match self.driver.write(&self.samples[self.offset..]) {
                Ok(0) | Err(Error(-11 | -4)) => return Ok(()),
                Ok(n) if n <= FRAMES - self.offset => self.offset += n,
                Ok(_) => return Err(Error(INTERNAL)),
                Err(Error(code @ (-32 | -86))) => match self.driver.recover(code) {
                    Ok(()) => {}
                    Err(Error(-11 | -4)) => return Ok(()),
                    Err(e) => return Err(e),
                },
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

/// PCM output pumped on its owning worker. Renderer panics become sticky errors
/// in unwind builds; renderers must not panic in abort builds.
#[cfg(target_os = "linux")]
pub struct Queue<F: FnMut(&mut [[f32; 2]]) + Send> {
    engine: Engine<F, device::Native>,
}
#[cfg(target_os = "linux")]
impl<F: FnMut(&mut [[f32; 2]]) + Send> Queue<F> {
    pub fn open(render: F) -> Result<Self, Error>
    where
        F: 'static,
    {
        unsafe { Self::open_scoped(render) }
    }
    pub fn open_device(name: &std::ffi::CStr, render: F) -> Result<Self, Error>
    where
        F: 'static,
    {
        Ok(Self {
            engine: Engine::open(device::Native::open(name)?, render)?,
        })
    }
    /// Dispose the queue before any borrowed renderer storage expires.
    pub(crate) unsafe fn open_scoped(render: F) -> Result<Self, Error> {
        use std::{ffi::CString, os::unix::ffi::OsStrExt};
        let name = std::env::var_os("KKRIEGER_AUDIO_DEVICE");
        let name = name
            .map(|s| CString::new(s.as_bytes()))
            .transpose()
            .map_err(|_| Error(-22))?;
        let name = name.as_deref().unwrap_or(c"default");
        Ok(Self {
            engine: Engine::open(device::Native::open(name)?, render)?,
        })
    }
    pub fn pump(&mut self) -> Result<(), Error> {
        self.engine.pump()
    }
    pub fn status(&self) -> i32 {
        self.engine.status
    }
}

#[cfg(target_os = "linux")]
mod device {
    use super::{Driver, Error, INTERNAL, UNAVAILABLE, recover_stream};
    use std::{
        ffi::{CStr, c_char, c_int, c_long, c_uint, c_ulong, c_void},
        ptr,
        sync::OnceLock,
    };
    type Handle = *mut c_void;
    type Open = unsafe extern "C" fn(*mut Handle, *const c_char, c_int, c_int) -> c_int;
    type SetParams =
        unsafe extern "C" fn(Handle, c_int, c_int, c_uint, c_uint, c_int, c_uint) -> c_int;
    type Write = unsafe extern "C" fn(Handle, *const c_void, c_ulong) -> c_long;
    type Control = unsafe extern "C" fn(Handle) -> c_int;
    struct Api {
        open: Open,
        set_params: SetParams,
        write: Write,
        prepare: Control,
        resume: Control,
        stop: Control,
        close: Control,
    }
    static API: OnceLock<Result<Api, Error>> = OnceLock::new();
    #[link(name = "dl")]
    unsafe extern "C" {
        fn dlopen(name: *const c_char, flags: c_int) -> Handle;
        fn dlsym(handle: Handle, name: *const c_char) -> Handle;
        fn dlclose(handle: Handle) -> c_int;
    }
    struct Library(Handle);
    impl Drop for Library {
        fn drop(&mut self) {
            unsafe {
                dlclose(self.0);
            }
        }
    }
    impl Library {
        fn symbol(&self, name: &CStr) -> Result<Handle, Error> {
            let p = unsafe { dlsym(self.0, name.as_ptr()) };
            if p.is_null() {
                Err(Error(UNAVAILABLE))
            } else {
                Ok(p)
            }
        }
    }
    impl Api {
        fn load() -> Result<Self, Error> {
            // RTLD_NOW | RTLD_LOCAL. Resolve the complete API before publishing it.
            let p = unsafe { dlopen(c"libasound.so.2".as_ptr(), 2) };
            if p.is_null() {
                return Err(Error(UNAVAILABLE));
            }
            let library = Library(p);
            // POSIX specifies conversion of dlsym results to function pointers.
            macro_rules! symbol {
                ($name:literal, $kind:ty) => {
                    unsafe { std::mem::transmute::<Handle, $kind>(library.symbol($name)?) }
                };
            }
            let api = Self {
                open: symbol!(c"snd_pcm_open", Open),
                set_params: symbol!(c"snd_pcm_set_params", SetParams),
                write: symbol!(c"snd_pcm_writei", Write),
                prepare: symbol!(c"snd_pcm_prepare", Control),
                resume: symbol!(c"snd_pcm_resume", Control),
                stop: symbol!(c"snd_pcm_drop", Control),
                close: symbol!(c"snd_pcm_close", Control),
            };
            // ALSA/plugin code stays loaded for the process lifetime. No Rust
            // callback or sample pointer is registered with the native library.
            std::mem::forget(library);
            Ok(api)
        }
    }
    pub(super) struct Native {
        api: &'static Api,
        pcm: Handle,
    }
    impl Native {
        pub(super) fn open(name: &CStr) -> Result<Self, Error> {
            let api = API.get_or_init(Api::load).as_ref().map_err(|e| *e)?;
            let mut pcm = ptr::null_mut();
            // Playback stream 0, SND_PCM_NONBLOCK 1.
            let code = unsafe { (api.open)(&mut pcm, name.as_ptr(), 0, 1) };
            if code < 0 {
                return Err(Error(code));
            }
            if pcm.is_null() {
                return Err(Error(INTERNAL));
            }
            let native = Self { api, pcm };
            // S16 CPU-endian, RW_INTERLEAVED 3, stereo 44100 Hz, ALSA's
            // normal rate conversion enabled, approximately 70 ms buffering.
            let format = if cfg!(target_endian = "little") { 2 } else { 3 };
            let code = unsafe { (api.set_params)(pcm, format, 3, 2, 44100, 1, 70000) };
            if code < 0 {
                return Err(Error(code));
            }
            Ok(native)
        }
    }
    impl Driver for Native {
        fn write(&mut self, samples: &[[i16; 2]]) -> Result<usize, Error> {
            let frames = c_ulong::try_from(samples.len()).map_err(|_| Error(INTERNAL))?;
            // writei consumes/copies this interleaved slice during the call.
            let n = unsafe { (self.api.write)(self.pcm, samples.as_ptr().cast(), frames) };
            if n < 0 {
                Err(Error(i32::try_from(n).unwrap_or(INTERNAL)))
            } else {
                usize::try_from(n).map_err(|_| Error(INTERNAL))
            }
        }
        fn recover(&mut self, code: i32) -> Result<(), Error> {
            recover_stream(
                code,
                || unsafe { (self.api.resume)(self.pcm) },
                || unsafe { (self.api.prepare)(self.pcm) },
            )
        }
    }
    impl Drop for Native {
        fn drop(&mut self) {
            // Stop rather than drain: shutdown must not wait for audio playback.
            // No driver-owned pointer refers to Rust sample/renderer storage.
            unsafe {
                (self.api.stop)(self.pcm);
                (self.api.close)(self.pcm);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, collections::VecDeque, rc::Rc};
    #[derive(Default)]
    struct State {
        writes: VecDeque<Result<usize, Error>>,
        recoveries: VecDeque<Result<(), Error>>,
        calls: Vec<usize>,
        recovery_codes: Vec<i32>,
        accepted: Vec<[i16; 2]>,
        dropped: usize,
    }
    struct Mock(Rc<RefCell<State>>);
    impl Driver for Mock {
        fn write(&mut self, samples: &[[i16; 2]]) -> Result<usize, Error> {
            let mut s = self.0.borrow_mut();
            s.calls.push(samples.len());
            let result = s.writes.pop_front().unwrap_or(Err(Error(-11)));
            if let Ok(n) = result
                && n <= samples.len()
            {
                s.accepted.extend_from_slice(&samples[..n]);
            }
            result
        }
        fn recover(&mut self, code: i32) -> Result<(), Error> {
            let mut s = self.0.borrow_mut();
            s.recovery_codes.push(code);
            s.recoveries.pop_front().unwrap_or(Ok(()))
        }
    }
    impl Drop for Mock {
        fn drop(&mut self) {
            self.0.borrow_mut().dropped += 1;
        }
    }
    fn state(writes: impl IntoIterator<Item = Result<usize, Error>>) -> Rc<RefCell<State>> {
        Rc::new(RefCell::new(State {
            writes: writes.into_iter().collect(),
            ..State::default()
        }))
    }
    #[test]
    fn partial_writes_preserve_the_unaccepted_samples() {
        let s = state([
            Ok(17),
            Err(Error(-11)),
            Ok(FRAMES - 17),
            Ok(FRAMES),
            Err(Error(-11)),
        ]);
        let mut renders = 0;
        let mut e = Engine::open(Mock(s.clone()), |out| {
            renders += 1;
            out.fill([renders as f32 / 8., -0.25]);
        })
        .unwrap();
        assert_eq!(e.offset, 17);
        e.pump().unwrap();
        assert_eq!(e.offset, 0);
        assert_eq!(
            s.borrow().calls,
            [FRAMES, FRAMES - 17, FRAMES - 17, FRAMES, FRAMES]
        );
        assert_eq!(
            s.borrow().accepted[..FRAMES],
            [[pcm16(0.125), pcm16(-0.25)]; FRAMES]
        );
        assert_eq!(
            s.borrow().accepted[FRAMES..],
            [[pcm16(0.25), pcm16(-0.25)]; FRAMES]
        );
        drop(e);
        assert_eq!(renders, 3);
        assert_eq!(s.borrow().dropped, 1);
    }
    #[test]
    fn zero_eagain_and_interrupt_return_without_spinning_or_rerendering() {
        let s = state([Ok(0), Err(Error(-11)), Err(Error(-4)), Ok(FRAMES)]);
        let mut renders = 0;
        let mut e = Engine::open(Mock(s.clone()), |out| {
            renders += 1;
            out.fill([0.; 2]);
        })
        .unwrap();
        for expected_calls in [2, 3] {
            e.pump().unwrap();
            assert_eq!(s.borrow().calls.len(), expected_calls);
        }
        assert_eq!(e.offset, 0);
        e.pump().unwrap();
        drop(e);
        assert_eq!(renders, 2);
    }
    #[test]
    fn recovering_keeps_pending_pcm_and_bounds_repeated_xruns() {
        let s = state([Err(Error(-32)), Err(Error(-86)), Ok(19), Err(Error(-11))]);
        let mut e = Engine::open(Mock(s.clone()), |out| out.fill([0.5, -0.5])).unwrap();
        assert_eq!(s.borrow().recovery_codes, [-32, -86]);
        assert_eq!(e.offset, 19);
        e.pump().unwrap();
        assert_eq!(e.offset, 19);
        assert_eq!(s.borrow().accepted, [[pcm16(0.5), pcm16(-0.5)]; 19]);
        s.borrow_mut().writes.extend([Err(Error(-32)); 6]);
        let before = s.borrow().calls.len();
        e.pump().unwrap();
        assert_eq!(s.borrow().calls.len() - before, 3);
    }
    #[test]
    fn suspended_resume_backpressure_retries_on_the_next_pump() {
        let s = state([Err(Error(-86)), Err(Error(-86)), Ok(FRAMES)]);
        s.borrow_mut().recoveries.extend([Err(Error(-11)), Ok(())]);
        let mut e = Engine::open(Mock(s.clone()), |out| out.fill([0.; 2])).unwrap();
        assert_eq!(s.borrow().calls.len(), 1);
        e.pump().unwrap();
        assert_eq!(s.borrow().accepted.len(), FRAMES);
        assert_eq!(s.borrow().recovery_codes, [-86, -86]);
    }
    #[test]
    fn fatal_write_and_recovery_failures_are_sticky() {
        for recover in [false, true] {
            let s = state([Err(Error(-11)), Err(Error(if recover { -32 } else { -19 }))]);
            if recover {
                s.borrow_mut().recoveries.push_back(Err(Error(-5)));
            }
            let mut e = Engine::open(Mock(s.clone()), |out| out.fill([0.; 2])).unwrap();
            let code = if recover { -5 } else { -19 };
            assert_eq!(e.pump(), Err(Error(code)));
            let calls = s.borrow().calls.len();
            assert_eq!(e.status, code);
            assert_eq!(e.pump(), Err(Error(code)));
            assert_eq!(s.borrow().calls.len(), calls);
            drop(e);
            assert_eq!(s.borrow().dropped, 1);
        }
    }
    #[test]
    fn invalid_driver_count_cannot_advance_past_pending_storage() {
        let s = state([Ok(FRAMES + 1)]);
        let result = Engine::open(Mock(s.clone()), |out| out.fill([0.; 2]));
        assert!(matches!(result, Err(Error(INTERNAL))));
        assert_eq!(s.borrow().dropped, 1);
    }
    #[test]
    fn priming_is_bounded_and_later_renderer_panics_become_sticky() {
        let s = state([Ok(FRAMES); 4]);
        let mut calls = 0;
        let mut e = Engine::open(Mock(s.clone()), |out| {
            calls += 1;
            if calls == 4 {
                panic!("renderer");
            }
            out.fill([0.; 2]);
        })
        .unwrap();
        assert_eq!(s.borrow().accepted.len(), 3 * FRAMES);
        assert_eq!(e.pump(), Err(Error(INTERNAL)));
        assert_eq!(e.pump(), Err(Error(INTERNAL)));
        assert_eq!(s.borrow().accepted.len(), 3 * FRAMES);
        drop(e);
        assert_eq!(s.borrow().dropped, 1);
    }
    #[test]
    fn native_recovery_policy_defers_resume_and_prepares_only_when_needed() {
        for code in [-32, -86] {
            for resumed in [0, -11, -4, -38] {
                for prepared in [0, -5, -11] {
                    let calls = RefCell::new(Vec::new());
                    let actual = recover_stream(
                        code,
                        || {
                            calls.borrow_mut().push("resume");
                            resumed
                        },
                        || {
                            calls.borrow_mut().push("prepare");
                            prepared
                        },
                    );
                    let should_prepare = code == -32 || !matches!(resumed, 0 | -11 | -4);
                    assert_eq!(
                        calls.into_inner(),
                        if code == -32 {
                            vec!["prepare"]
                        } else if should_prepare {
                            vec!["resume", "prepare"]
                        } else {
                            vec!["resume"]
                        }
                    );
                    let result = if should_prepare { prepared } else { resumed };
                    assert_eq!(
                        actual,
                        if result < 0 {
                            Err(Error(result))
                        } else {
                            Ok(())
                        }
                    );
                }
            }
        }
    }
    #[test]
    fn priming_failure_or_panic_disposes_the_driver() {
        let s = state([Err(Error(-19))]);
        let r = Engine::open(Mock(s.clone()), |out| out.fill([0.; 2]));
        assert!(matches!(r, Err(Error(-19))));
        assert_eq!(s.borrow().dropped, 1);
        let s = state([]);
        let r = Engine::open(Mock(s.clone()), |_| panic!("startup renderer"));
        assert!(matches!(r, Err(Error(INTERNAL))));
        assert_eq!(s.borrow().dropped, 1);
    }
}

#[cfg(all(test, target_os = "linux"))]
mod native_tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    #[test]
    fn alsa_null_device_accepts_pcm_and_disposes_the_renderer() {
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = calls.clone();
        let mut queue = Queue::open_device(c"null", move |out| {
            assert_eq!(out.len(), FRAMES);
            out.fill([0.125, -0.25]);
            observed.fetch_add(1, Ordering::Relaxed);
        })
        .expect("libasound.so.2 and ALSA null PCM must be available");
        let priming = calls.load(Ordering::Relaxed);
        assert!(priming > 0);
        for _ in 0..8 {
            queue.pump().unwrap();
        }
        assert!(calls.load(Ordering::Relaxed) > priming);
        assert_eq!(queue.status(), 0);
        drop(queue);
        assert_eq!(Arc::strong_count(&calls), 1);
    }
}
