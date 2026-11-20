//! WinMM output; rendering stays on the owning music worker.

use crate::audio_pcm::{BYTES, FRAMES, pcm16};
use std::{
    cell::UnsafeCell,
    panic::{AssertUnwindSafe, catch_unwind},
    ptr,
    sync::atomic::{AtomicU32, Ordering},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Error(pub i32);
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Windows waveform audio error {}", self.0)
    }
}
impl std::error::Error for Error {}
#[repr(C, packed)]
struct Format {
    tag: u16,
    channels: u16,
    rate: u32,
    byte_rate: u32,
    align: u16,
    bits: u16,
    extra: u16,
}
impl Format {
    fn pcm() -> Self {
        Self {
            tag: 1,
            channels: 2,
            rate: 44100,
            byte_rate: 176400,
            align: 4,
            bits: 16,
            extra: 0,
        }
    }
}
#[repr(C)]
struct Header {
    data: *mut u8,
    length: u32,
    recorded: u32,
    user: usize,
    flags: u32,
    loops: u32,
    next: *mut Header,
    reserved: usize,
}
// Check the native ABI during cross-compilation as well as host tests.
const _: () = {
    assert!(size_of::<Format>() == 18);
    assert!(align_of::<Header>() >= align_of::<AtomicU32>());
    assert!(std::mem::offset_of!(Header, flags) % align_of::<AtomicU32>() == 0);
    assert!(size_of::<Header>() == if size_of::<usize>() == 8 { 48 } else { 32 });
    assert!(std::mem::offset_of!(Header, flags) == if size_of::<usize>() == 8 { 24 } else { 16 });
};

#[repr(C)]
struct Buffer {
    header: UnsafeCell<Header>,
    samples: UnsafeCell<[[i16; 2]; FRAMES]>,
    prepared: bool,
}
impl Buffer {
    fn new(index: usize) -> Box<Self> {
        let b = Box::new(Self {
            header: UnsafeCell::new(Header {
                data: ptr::null_mut(),
                length: BYTES,
                recorded: 0,
                user: index,
                flags: 0,
                loops: 0,
                next: ptr::null_mut(),
                reserved: 0,
            }),
            samples: UnsafeCell::new([[0; 2]; FRAMES]),
            prepared: false,
        });
        // Header and PCM addresses stay pinned while WinMM can reference them.
        unsafe {
            (*b.header.get()).data = b.samples.get().cast();
        }
        b
    }
}
/// # Safety
/// Prepared headers remain valid until unprepare succeeds. Writes borrow PCM
/// until the driver publishes DONE through the aligned atomic DWORD flags.
/// Reset returns all buffers; close releases all native references.
unsafe trait Driver {
    fn pause(&mut self) -> Result<(), Error>;
    unsafe fn prepare(&mut self, header: *mut Header) -> Result<(), Error>;
    unsafe fn write(&mut self, header: *mut Header) -> Result<(), Error>;
    fn restart(&mut self) -> Result<(), Error>;
    fn ready(&mut self) -> Result<bool, Error>;
    fn reset(&mut self) -> Result<(), Error>;
    unsafe fn unprepare(&mut self, header: *mut Header) -> Result<(), Error>;
    fn close(&mut self) -> Result<(), Error>;
}
struct Engine<F: FnMut(&mut [[f32; 2]]), D: Driver> {
    driver: Option<D>,
    render: F,
    scratch: [[f32; 2]; FRAMES],
    buffers: [Box<Buffer>; 3],
    status: i32,
}
impl<F: FnMut(&mut [[f32; 2]]), D: Driver> Engine<F, D> {
    fn open(driver: D, render: F) -> Result<Self, Error> {
        let mut e = Self {
            driver: Some(driver),
            render,
            scratch: [[0.; 2]; FRAMES],
            buffers: std::array::from_fn(Buffer::new),
            status: 0,
        };
        e.driver.as_mut().unwrap().pause()?;
        for buffer in &mut e.buffers {
            // A successful prepare pins the header until reset/unprepare below.
            unsafe {
                e.driver.as_mut().unwrap().prepare(buffer.header.get())?;
            }
            buffer.prepared = true;
            Self::fill(&mut e.render, &mut e.scratch, buffer)?;
            unsafe {
                e.driver.as_mut().unwrap().write(buffer.header.get())?;
            }
        }
        e.driver.as_mut().unwrap().restart()?;
        Ok(e)
    }
    fn fill(
        render: &mut F,
        scratch: &mut [[f32; 2]; FRAMES],
        buffer: &Buffer,
    ) -> Result<(), Error> {
        // Only called before submission or after WHDR_DONE returns ownership.
        let samples = unsafe { &mut *buffer.samples.get() };
        samples.fill([0; 2]);
        scratch.fill([0.; 2]);
        catch_unwind(AssertUnwindSafe(|| render(scratch))).map_err(|_| Error(i32::MIN))?;
        for (out, input) in samples.iter_mut().zip(scratch) {
            *out = input.map(pcm16);
        }
        Ok(())
    }
    fn pump(&mut self) -> Result<(), Error> {
        if self.status != 0 {
            return Err(Error(self.status));
        }
        let result = (|| {
            let driver = self.driver.as_mut().ok_or(Error(i32::MIN))?;
            if !driver.ready()? {
                return Ok(());
            }
            for buffer in &self.buffers {
                // Win32 guarantees atomic aligned DWORD access. Acquire observes
                // the driver's DONE ownership handoff without a racing Rust load.
                let done =
                    unsafe { AtomicU32::from_ptr(ptr::addr_of_mut!((*buffer.header.get()).flags)) }
                        .load(Ordering::Acquire)
                        & 1
                        != 0;
                if done {
                    Self::fill(&mut self.render, &mut self.scratch, buffer)?;
                    unsafe {
                        driver.write(buffer.header.get())?;
                    }
                }
            }
            Ok(())
        })();
        if let Err(Error(code)) = result {
            self.status = code;
        }
        result
    }
    fn shutdown(&mut self) -> Result<(), Error> {
        if let Some(driver) = self.driver.as_mut() {
            driver.reset()?;
            for buffer in &mut self.buffers {
                if buffer.prepared {
                    unsafe {
                        driver.unprepare(buffer.header.get())?;
                    }
                    buffer.prepared = false;
                }
            }
            driver.close()?;
            self.driver = None;
        }
        Ok(())
    }
}
impl<F: FnMut(&mut [[f32; 2]]), D: Driver> Drop for Engine<F, D> {
    fn drop(&mut self) {
        // Returning after failed cleanup could free driver-owned PCM/header memory.
        if self.shutdown().is_err() {
            std::process::abort();
        }
    }
}

/// Pump on the owning audio worker. Renderer panics report i32::MIN when
/// unwinding is enabled; renderers must not panic in abort builds.
#[cfg(target_os = "windows")]
pub struct Queue<F: FnMut(&mut [[f32; 2]]) + Send> {
    engine: Engine<F, device::Native>,
}
#[cfg(target_os = "windows")]
impl<F: FnMut(&mut [[f32; 2]]) + Send> Queue<F> {
    pub fn open(render: F) -> Result<Self, Error>
    where
        F: 'static,
    {
        unsafe { Self::open_scoped(render) }
    }
    /// Caller must dispose this queue before borrowed renderer storage expires.
    pub(crate) unsafe fn open_scoped(render: F) -> Result<Self, Error> {
        Ok(Self {
            engine: Engine::open(device::Native::open()?, render)?,
        })
    }
    /// Refill returned buffers on the owning audio worker, never an FFI callback.
    pub fn pump(&mut self) -> Result<(), Error> {
        self.engine.pump()
    }
    pub fn status(&self) -> i32 {
        self.engine.status
    }
}

#[cfg(target_os = "windows")]
mod device {
    use super::{Driver, Error, Format, Header};
    use std::{ffi::c_void, ptr};
    type Handle = *mut c_void;
    #[link(name = "winmm")]
    unsafe extern "system" {
        #[link_name = "waveOutOpen"]
        fn wave_open(
            out: *mut Handle,
            id: u32,
            format: *const Format,
            callback: usize,
            instance: usize,
            flags: u32,
        ) -> u32;
        #[link_name = "waveOutPause"]
        fn wave_pause(device: Handle) -> u32;
        #[link_name = "waveOutRestart"]
        fn wave_restart(device: Handle) -> u32;
        #[link_name = "waveOutPrepareHeader"]
        fn wave_prepare(device: Handle, header: *mut Header, bytes: u32) -> u32;
        #[link_name = "waveOutWrite"]
        fn wave_write(device: Handle, header: *mut Header, bytes: u32) -> u32;
        #[link_name = "waveOutReset"]
        fn wave_reset(device: Handle) -> u32;
        #[link_name = "waveOutUnprepareHeader"]
        fn wave_unprepare(device: Handle, header: *mut Header, bytes: u32) -> u32;
        #[link_name = "waveOutClose"]
        fn wave_close(device: Handle) -> u32;
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        #[link_name = "CreateEventW"]
        fn create_event(
            attributes: *const c_void,
            manual: i32,
            initial: i32,
            name: *const u16,
        ) -> Handle;
        #[link_name = "WaitForSingleObject"]
        fn wait(event: Handle, milliseconds: u32) -> u32;
        #[link_name = "CloseHandle"]
        fn close_handle(event: Handle) -> i32;
        #[link_name = "GetLastError"]
        fn last_error() -> u32;
    }
    fn check(code: u32) -> Result<(), Error> {
        if code == 0 {
            Ok(())
        } else {
            Err(Error(code as i32))
        }
    }
    fn system_error() -> Error {
        let code = unsafe { last_error() };
        Error(if code == 0 { i32::MIN } else { code as i32 })
    }
    pub(super) struct Native {
        wave: Handle,
        event: Handle,
    }
    impl Native {
        pub(super) fn open() -> Result<Self, Error> {
            let mut driver = Self {
                wave: ptr::null_mut(),
                event: unsafe { create_event(ptr::null(), 0, 0, ptr::null()) },
            };
            if driver.event.is_null() {
                return Err(system_error());
            }
            // Unnamed auto-reset event: no Rust callbacks or borrowed user pointer.
            let status = unsafe {
                wave_open(
                    &mut driver.wave,
                    u32::MAX,
                    &Format::pcm(),
                    driver.event as usize,
                    0,
                    0x50000,
                )
            };
            if status != 0 {
                driver.wave = ptr::null_mut();
                return Err(Error(status as i32));
            }
            Ok(driver)
        }
    }
    unsafe impl Driver for Native {
        fn pause(&mut self) -> Result<(), Error> {
            check(unsafe { wave_pause(self.wave) })
        }
        unsafe fn prepare(&mut self, header: *mut Header) -> Result<(), Error> {
            check(unsafe { wave_prepare(self.wave, header, size_of::<Header>() as u32) })
        }
        unsafe fn write(&mut self, header: *mut Header) -> Result<(), Error> {
            check(unsafe { wave_write(self.wave, header, size_of::<Header>() as u32) })
        }
        fn restart(&mut self) -> Result<(), Error> {
            check(unsafe { wave_restart(self.wave) })
        }
        fn ready(&mut self) -> Result<bool, Error> {
            match unsafe { wait(self.event, 0) } {
                0 => Ok(true),
                0x102 => Ok(false),
                _ => Err(system_error()),
            }
        }
        fn reset(&mut self) -> Result<(), Error> {
            if self.wave.is_null() {
                Ok(())
            } else {
                check(unsafe { wave_reset(self.wave) })
            }
        }
        unsafe fn unprepare(&mut self, header: *mut Header) -> Result<(), Error> {
            check(unsafe { wave_unprepare(self.wave, header, size_of::<Header>() as u32) })
        }
        fn close(&mut self) -> Result<(), Error> {
            if !self.wave.is_null() {
                check(unsafe { wave_close(self.wave) })?;
                self.wave = ptr::null_mut();
            }
            if !self.event.is_null() {
                if unsafe { close_handle(self.event) } == 0 {
                    return Err(system_error());
                }
                self.event = ptr::null_mut();
            }
            Ok(())
        }
    }
    impl Drop for Native {
        fn drop(&mut self) {
            if self.close().is_err() {
                std::process::abort();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};
    #[derive(Clone, Debug, PartialEq, Eq)]
    enum Call {
        Pause,
        Prepare(usize),
        Write(usize),
        Restart,
        Wait,
        Reset,
        Unprepare(usize),
        Close,
    }
    struct State {
        calls: Vec<Call>,
        headers: [*mut Header; 3],
        samples: Vec<(usize, [i16; 2])>,
        ready: bool,
        fail: Option<usize>,
    }
    impl Default for State {
        fn default() -> Self {
            Self {
                calls: Vec::new(),
                headers: [std::ptr::null_mut(); 3],
                samples: Vec::new(),
                ready: false,
                fail: None,
            }
        }
    }
    impl State {
        fn record(&mut self, call: Call) -> Result<(), Error> {
            let index = self.calls.len();
            self.calls.push(call);
            if self.fail == Some(index) {
                self.fail = None;
                Err(Error(77))
            } else {
                Ok(())
            }
        }
        fn complete(&mut self, index: usize) {
            // Simulate the documented ownership handoff before signaling.
            unsafe {
                (*self.headers[index]).flags = ((*self.headers[index]).flags & !16) | 1;
            }
            self.ready = true;
        }
    }
    struct Mock(Rc<RefCell<State>>);
    unsafe impl Driver for Mock {
        fn pause(&mut self) -> Result<(), Error> {
            self.0.borrow_mut().record(Call::Pause)
        }
        unsafe fn prepare(&mut self, header: *mut Header) -> Result<(), Error> {
            let mut s = self.0.borrow_mut();
            let index = unsafe { (*header).user };
            s.record(Call::Prepare(index))?;
            s.headers[index] = header;
            unsafe { (*header).flags = 2 };
            Ok(())
        }
        unsafe fn write(&mut self, header: *mut Header) -> Result<(), Error> {
            let mut s = self.0.borrow_mut();
            let index = unsafe { (*header).user };
            s.record(Call::Write(index))?;
            unsafe {
                assert_eq!((*header).length, BYTES);
                assert_ne!((*header).flags & 2, 0);
                s.samples.push((index, *(*header).data.cast::<[i16; 2]>()));
                (*header).flags = 18;
            }
            Ok(())
        }
        fn restart(&mut self) -> Result<(), Error> {
            self.0.borrow_mut().record(Call::Restart)
        }
        fn ready(&mut self) -> Result<bool, Error> {
            let mut s = self.0.borrow_mut();
            s.record(Call::Wait)?;
            Ok(std::mem::take(&mut s.ready))
        }
        fn reset(&mut self) -> Result<(), Error> {
            let mut s = self.0.borrow_mut();
            s.record(Call::Reset)?;
            for header in s.headers {
                if !header.is_null() {
                    unsafe {
                        (*header).flags = ((*header).flags & !16) | 1;
                    }
                }
            }
            Ok(())
        }
        unsafe fn unprepare(&mut self, header: *mut Header) -> Result<(), Error> {
            let mut s = self.0.borrow_mut();
            let index = unsafe { (*header).user };
            s.record(Call::Unprepare(index))?;
            unsafe {
                assert_eq!((*header).flags & 16, 0);
                assert!(!(*header).data.is_null());
                (*header).flags = 0;
            }
            Ok(())
        }
        fn close(&mut self) -> Result<(), Error> {
            let mut s = self.0.borrow_mut();
            s.record(Call::Close)?;
            for h in s.headers {
                if !h.is_null() {
                    unsafe {
                        assert_eq!((*h).flags & 18, 0);
                    }
                }
            }
            Ok(())
        }
    }
    #[test]
    fn format_and_header_layout_match_winmm_abi() {
        assert_eq!(size_of::<Format>(), 18);
        assert_eq!(
            size_of::<Header>(),
            if size_of::<usize>() == 8 { 48 } else { 32 }
        );
        assert_eq!(
            std::mem::offset_of!(Header, flags),
            if size_of::<usize>() == 8 { 24 } else { 16 }
        );
        let f = Format::pcm();
        assert_eq!(
            (
                f.tag,
                f.channels,
                f.rate,
                f.byte_rate,
                f.align,
                f.bits,
                f.extra
            ),
            (1, 2, 44100, 176400, 4, 16, 0)
        );
    }
    #[test]
    fn primes_all_buffers_while_paused_and_only_refills_returned_storage() {
        let s = Rc::new(RefCell::new(State::default()));
        let calls = Rc::new(RefCell::new(0));
        let count = Rc::clone(&calls);
        let engine = Engine::open(Mock(Rc::clone(&s)), move |out| {
            *count.borrow_mut() += 1;
            out.fill([0.5, -0.5]);
        })
        .unwrap();
        let mut engine = Box::new(engine);
        let addresses = s.borrow().headers;
        assert_eq!(*calls.borrow(), 3);
        assert_eq!(
            s.borrow().samples,
            vec![
                (0, [16384, -16384]),
                (1, [16384, -16384]),
                (2, [16384, -16384])
            ]
        );
        assert_eq!(
            s.borrow().calls,
            vec![
                Call::Pause,
                Call::Prepare(0),
                Call::Write(0),
                Call::Prepare(1),
                Call::Write(1),
                Call::Prepare(2),
                Call::Write(2),
                Call::Restart
            ]
        );
        engine.pump().unwrap();
        assert_eq!(*calls.borrow(), 3);
        s.borrow_mut().complete(1);
        engine.pump().unwrap();
        assert_eq!(*calls.borrow(), 4);
        s.borrow_mut().complete(0);
        s.borrow_mut().complete(2);
        engine.pump().unwrap();
        assert_eq!(*calls.borrow(), 6);
        assert_eq!(s.borrow().headers, addresses);
        drop(engine);
        assert!(s.borrow().calls.ends_with(&[
            Call::Reset,
            Call::Unprepare(0),
            Call::Unprepare(1),
            Call::Unprepare(2),
            Call::Close
        ]));
    }
    #[test]
    fn every_initialization_failure_releases_only_prepared_headers_before_close() {
        for fail in 0..8 {
            let s = Rc::new(RefCell::new(State {
                fail: Some(fail),
                ..Default::default()
            }));
            let result = Engine::open(Mock(Rc::clone(&s)), |out| out.fill([0.; 2]));
            assert!(matches!(result, Err(Error(77))), "failure {fail}");
            let s = s.borrow();
            assert_eq!(s.calls.last(), Some(&Call::Close));
            let prepared = s
                .calls
                .iter()
                .take(fail)
                .filter(|c| matches!(c, Call::Prepare(_)))
                .count();
            assert_eq!(
                s.calls
                    .iter()
                    .filter(|c| matches!(c, Call::Unprepare(_)))
                    .count(),
                prepared
            );
            assert!(s.calls.contains(&Call::Reset));
        }
    }
    #[test]
    fn renderer_failure_latches_status_and_does_not_requeue_the_buffer() {
        let s = Rc::new(RefCell::new(State::default()));
        let mut count = 0;
        let mut e = Engine::open(Mock(Rc::clone(&s)), move |out| {
            count += 1;
            if count == 4 {
                panic!("renderer failed")
            };
            out.fill([0.; 2]);
        })
        .unwrap();
        s.borrow_mut().complete(0);
        assert_eq!(e.pump(), Err(Error(i32::MIN)));
        assert_eq!(e.status, i32::MIN);
        assert_eq!(s.borrow().samples.len(), 3);
        let n = s.borrow().calls.len();
        assert_eq!(e.pump(), Err(Error(i32::MIN)));
        assert_eq!(s.borrow().calls.len(), n);
    }
    #[test]
    fn failed_shutdown_retains_driver_owned_memory_until_successful_release() {
        let s = Rc::new(RefCell::new(State::default()));
        let mut e = Engine::open(Mock(Rc::clone(&s)), |out| out.fill([0.; 2])).unwrap();
        let next = s.borrow().calls.len();
        s.borrow_mut().fail = Some(next);
        assert_eq!(e.shutdown(), Err(Error(77)));
        assert!(e.driver.is_some());
        assert!(e.buffers.iter().all(|b| b.prepared));
        e.shutdown().unwrap();
        assert!(e.driver.is_none());
    }
    #[test]
    fn spurious_notifications_do_not_recycle_in_flight_buffers() {
        let s = Rc::new(RefCell::new(State::default()));
        let mut e = Engine::open(Mock(Rc::clone(&s)), |out| out.fill([0.; 2])).unwrap();
        s.borrow_mut().ready = true;
        e.pump().unwrap();
        assert_eq!(s.borrow().samples.len(), 3);
    }
    #[test]
    fn refill_api_failures_latch_without_repeating_native_calls() {
        for fail in [0, 1] {
            let s = Rc::new(RefCell::new(State::default()));
            let mut e = Engine::open(Mock(Rc::clone(&s)), |out| out.fill([0.; 2])).unwrap();
            let base = s.borrow().calls.len();
            s.borrow_mut().fail = Some(base + fail);
            s.borrow_mut().complete(0);
            assert_eq!(e.pump(), Err(Error(77)));
            assert_eq!(e.status, 77);
            let calls = s.borrow().calls.len();
            assert_eq!(e.pump(), Err(Error(77)));
            assert_eq!(s.borrow().calls.len(), calls);
        }
    }
    #[test]
    fn each_shutdown_failure_preserves_pinned_storage_for_retry() {
        for fail in 0..5 {
            let s = Rc::new(RefCell::new(State::default()));
            let mut e = Engine::open(Mock(Rc::clone(&s)), |out| out.fill([0.; 2])).unwrap();
            let pointers = s.borrow().headers;
            let base = s.borrow().calls.len();
            s.borrow_mut().fail = Some(base + fail);
            assert_eq!(e.shutdown(), Err(Error(77)));
            assert!(e.driver.is_some());
            for (buffer, pointer) in e.buffers.iter().zip(pointers) {
                assert_eq!(buffer.header.get(), pointer);
                unsafe {
                    assert_eq!((*pointer).data, buffer.samples.get().cast());
                    assert_eq!((*pointer).length, BYTES);
                }
            }
            e.shutdown().unwrap();
            assert!(e.driver.is_none());
        }
    }
    #[test]
    fn renderer_failure_during_each_priming_block_releases_prepared_storage() {
        for fail in 1..=3 {
            let s = Rc::new(RefCell::new(State::default()));
            let mut count = 0;
            let result = Engine::open(Mock(Rc::clone(&s)), move |out| {
                count += 1;
                if count == fail {
                    panic!("priming failure")
                };
                out.fill([0.; 2]);
            });
            assert!(matches!(result, Err(Error(i32::MIN))));
            let s = s.borrow();
            assert_eq!(s.samples.len(), fail - 1);
            assert_eq!(
                s.calls
                    .iter()
                    .filter(|c| matches!(c, Call::Unprepare(_)))
                    .count(),
                fail
            );
            assert_eq!(s.calls.last(), Some(&Call::Close));
        }
    }
}
