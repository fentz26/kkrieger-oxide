//! Native macOS output for the original 44,100 Hz stereo mix.
//! Audio Queue owns its three PCM buffers; callback state survives synchronous disposal.
//! https://developer.apple.com/documentation/audiotoolbox/audio-queue-services
use crate::audio_pcm::{BYTES, FRAMES, pcm16};
use std::{
    ffi::c_void,
    panic::{AssertUnwindSafe, catch_unwind},
    ptr::{self, NonNull},
    sync::{
        Mutex,
        atomic::{AtomicI32, Ordering},
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Error(pub i32);
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Audio Queue error {}", self.0)
    }
}
impl std::error::Error for Error {}
#[repr(C)]
struct Format {
    sample_rate: f64,
    format: u32,
    flags: u32,
    bytes_per_packet: u32,
    frames_per_packet: u32,
    bytes_per_frame: u32,
    channels: u32,
    bits: u32,
    reserved: u32,
}
#[repr(C)]
struct Buffer {
    capacity: u32,
    data: *mut c_void,
    size: u32,
    user_data: *mut c_void,
    packet_capacity: u32,
    packets: *mut c_void,
    packet_count: u32,
}
type RawQueue = *mut c_void;
#[link(name = "AudioToolbox", kind = "framework")]
unsafe extern "C" {
    fn AudioQueueNewOutput(
        format: *const Format,
        callback: unsafe extern "C" fn(*mut c_void, RawQueue, *mut Buffer),
        user: *mut c_void,
        run_loop: *const c_void,
        run_mode: *const c_void,
        flags: u32,
        queue: *mut RawQueue,
    ) -> i32;
    fn AudioQueueAllocateBuffer(queue: RawQueue, bytes: u32, buffer: *mut *mut Buffer) -> i32;
    fn AudioQueueEnqueueBuffer(
        queue: RawQueue,
        buffer: *mut Buffer,
        packets: u32,
        descriptions: *const c_void,
    ) -> i32;
    fn AudioQueueStart(queue: RawQueue, start_time: *const c_void) -> i32;
    fn AudioQueueDispose(queue: RawQueue, immediate: u8) -> i32;
}
struct RenderState<F> {
    callback: F,
    scratch: [[f32; 2]; FRAMES],
}
struct Callback<F> {
    render: Mutex<RenderState<F>>,
    status: AtomicI32,
}
/// The renderer runs on an Audio Queue thread after the three initial buffers
/// are primed. It must finish promptly and must not block on the game thread.
/// Dropping the queue synchronously stops callbacks before releasing the renderer.
pub struct Queue<F: FnMut(&mut [[f32; 2]]) + Send> {
    raw: RawQueue,
    callback: Box<Callback<F>>,
}
impl<F: FnMut(&mut [[f32; 2]]) + Send> Queue<F> {
    pub fn open(render: F) -> Result<Self, Error>
    where
        F: 'static,
    {
        // Static callback data remains valid even if a caller forgets the queue.
        unsafe { Self::open_scoped(render) }
    }
    /// Caller must dispose the queue before borrowed renderer storage expires.
    /// Unlike open(), forgetting a borrowed queue would violate this contract.
    pub(crate) unsafe fn open_scoped(render: F) -> Result<Self, Error> {
        let mut queue = Self {
            raw: ptr::null_mut(),
            callback: Box::new(Callback {
                render: Mutex::new(RenderState {
                    callback: render,
                    scratch: [[0.0; 2]; FRAMES],
                }),
                status: AtomicI32::new(0),
            }),
        };
        let format = Format {
            sample_rate: 44100.0,
            format: u32::from_be_bytes(*b"lpcm"),
            // Signed integer | packed; native little-endian stereo PCM16.
            flags: 12,
            bytes_per_packet: 4,
            frames_per_packet: 1,
            bytes_per_frame: 4,
            channels: 2,
            bits: 16,
            reserved: 0,
        };
        // All pointers stay valid through synchronous disposal. NULL run loop
        // selects the queue's internal callback thread; flags are reserved zero.
        unsafe {
            check(AudioQueueNewOutput(
                &format,
                refill::<F>,
                ptr::from_mut(&mut *queue.callback).cast(),
                ptr::null(),
                ptr::null(),
                0,
                &mut queue.raw,
            ))?;
            if queue.raw.is_null() {
                return Err(Error(i32::MIN));
            }
            for _ in 0..3 {
                let mut buffer = ptr::null_mut();
                check(AudioQueueAllocateBuffer(queue.raw, BYTES, &mut buffer))?;
                refill::<F>(
                    ptr::from_mut(&mut *queue.callback).cast(),
                    queue.raw,
                    buffer,
                );
                check(queue.status())?;
            }
            check(AudioQueueStart(queue.raw, ptr::null()))?;
        }
        Ok(queue)
    }
    /// First callback failure, or zero. A panicking renderer is silenced and
    /// reports i32::MIN when panic unwinding is enabled. Renderers must not panic
    /// in abort builds. Native API failures retain their OSStatus value.
    pub fn status(&self) -> i32 {
        self.callback.status.load(Ordering::Acquire)
    }
}
impl<F: FnMut(&mut [[f32; 2]]) + Send> Drop for Queue<F> {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            // AudioQueueDispose(true) releases buffers and waits for callbacks.
            let status = unsafe { AudioQueueDispose(self.raw, 1) };
            if status != 0 {
                // A renderer may borrow caller storage. Returning after a failed
                // disposal could leave native callbacks referencing freed data.
                std::process::abort();
            }
        }
    }
}
fn check(status: i32) -> Result<(), Error> {
    if status == 0 {
        Ok(())
    } else {
        Err(Error(status))
    }
}
unsafe extern "C" fn refill<F: FnMut(&mut [[f32; 2]]) + Send>(
    user: *mut c_void,
    queue: RawQueue,
    buffer: *mut Buffer,
) {
    // The API supplies its allocated buffer and the boxed state provided at
    // creation. Mutex prevents simultaneous mutable access if callbacks overlap.
    unsafe {
        let state = &*user.cast::<Callback<F>>();
        let Some(mut buffer) = NonNull::new(buffer) else {
            let _ = state
                .status
                .compare_exchange(0, i32::MIN, Ordering::AcqRel, Ordering::Acquire);
            return;
        };
        let buffer = buffer.as_mut();
        if buffer.capacity < BYTES || buffer.data.is_null() {
            let _ = state
                .status
                .compare_exchange(0, i32::MIN, Ordering::AcqRel, Ordering::Acquire);
            return;
        }
        let frames = std::slice::from_raw_parts_mut(buffer.data.cast::<[i16; 2]>(), FRAMES);
        frames.fill([0; 2]);
        if state.status.load(Ordering::Acquire) == 0 {
            let result = catch_unwind(AssertUnwindSafe(|| {
                let mut render = state.render.try_lock().map_err(|_| ())?;
                let RenderState { callback, scratch } = &mut *render;
                scratch.fill([0.0; 2]);
                callback(scratch);
                for (frame, source) in frames.iter_mut().zip(scratch) {
                    *frame = source.map(pcm16);
                }
                Ok::<(), ()>(())
            }));
            if !matches!(result, Ok(Ok(()))) {
                frames.fill([0; 2]);
                let _ =
                    state
                        .status
                        .compare_exchange(0, i32::MIN, Ordering::AcqRel, Ordering::Acquire);
            }
        }
        buffer.size = BYTES;
        let result = AudioQueueEnqueueBuffer(queue, buffer, 0, ptr::null());
        if result != 0 {
            let _ = state
                .status
                .compare_exchange(0, result, Ordering::AcqRel, Ordering::Acquire);
        }
    }
}
