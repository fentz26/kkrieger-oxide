//! Size probe: the platform services kkrieger needs on Linux, called directly
//! (Xlib window/input, GLX clear+swap, ALSA PCM16), with no std.
#![no_std]
#![no_main]
use core::{ffi::c_void, ptr::null_mut};
type P = *mut c_void;

#[link(name = "c")]
unsafe extern "C" {
    fn exit(code: i32) -> !;
}
#[link(name = "X11")]
unsafe extern "C" {
    fn XOpenDisplay(name: *const u8) -> P;
    fn XDefaultRootWindow(d: P) -> usize;
    fn XCreateWindow(d: P, parent: usize, x: i32, y: i32, w: u32, h: u32, b: u32, depth: i32, class: u32, visual: P, mask: usize, attrs: *mut [usize; 15]) -> usize;
    fn XCreateColormap(d: P, w: usize, visual: P, alloc: i32) -> usize;
    fn XMapWindow(d: P, w: usize) -> i32;
    fn XPending(d: P) -> i32;
    fn XNextEvent(d: P, e: *mut [usize; 24]) -> i32;
    fn XLookupKeysym(e: *mut [usize; 24], i: i32) -> usize;
    fn XWarpPointer(d: P, s: usize, w: usize, sx: i32, sy: i32, sw: u32, sh: u32, x: i32, y: i32) -> i32;
}
#[link(name = "GL")]
unsafe extern "C" {
    fn glXChooseVisual(d: P, screen: i32, attrs: *const i32) -> *mut XVisualInfo;
    fn glXCreateContext(d: P, vi: *mut XVisualInfo, share: P, direct: i32) -> P;
    fn glXMakeCurrent(d: P, w: usize, ctx: P) -> i32;
    fn glXSwapBuffers(d: P, w: usize);
    fn glXGetProcAddress(name: *const u8) -> P;
    fn glClearColor(r: f32, g: f32, b: f32, a: f32);
    fn glClear(mask: u32);
}
#[link(name = "asound")]
unsafe extern "C" {
    fn snd_pcm_open(pcm: *mut P, name: *const u8, stream: i32, mode: i32) -> i32;
    fn snd_pcm_set_params(pcm: P, fmt: i32, access: i32, ch: u32, rate: u32, resample: i32, latency: u32) -> i32;
    fn snd_pcm_writei(pcm: P, buf: *const i16, frames: usize) -> isize;
}
#[repr(C)]
struct XVisualInfo { visual: P, id: usize, screen: i32, depth: i32 }

static mut AUDIO: [i16; 2 * 1024] = [0; 2 * 1024];

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    unsafe {
        let d = XOpenDisplay(null_mut());
        if d.is_null() { exit(3) }
        let root = XDefaultRootWindow(d);
        let vi = glXChooseVisual(d, 0, [4, 12, 24, 5, 0].as_ptr());
        if vi.is_null() { exit(4) }
        let mut attrs = [0usize; 15];
        attrs[13] = XCreateColormap(d, root, (*vi).visual, 0);
        attrs[10] = 0x3 | 0x4 | 0x40; // key press/release, button press, pointer motion
        let w = XCreateWindow(d, root, 0, 0, 800, 600, 0, (*vi).depth, 1, (*vi).visual, 0x2000 | 0x800, &mut attrs);
        XMapWindow(d, w);
        let gl = glXCreateContext(d, vi, null_mut(), 1);
        glXMakeCurrent(d, w, gl);
        // Shader-era entry points are loaded at runtime, as a renderer would.
        let _ = glXGetProcAddress(b"glCreateShader\0".as_ptr());
        let mut pcm = null_mut();
        let audio = snd_pcm_open(&mut pcm, b"default\0".as_ptr(), 0, 1) == 0
            && snd_pcm_set_params(pcm, 2, 3, 2, 44100, 1, 50000) == 0;
        let mut event = [0usize; 24];
        let mut frame = 0u32;
        loop {
            while XPending(d) > 0 {
                XNextEvent(d, &mut event);
                if event[0] as u32 == 2 && XLookupKeysym(&mut event, 0) == 0xff1b { exit(0) }
            }
            XWarpPointer(d, 0, w, 0, 0, 0, 0, 400, 300);
            glClearColor((frame & 255) as f32 / 255., 0., 0., 1.);
            glClear(0x4000);
            glXSwapBuffers(d, w);
            if audio { snd_pcm_writei(pcm, core::ptr::addr_of!(AUDIO).cast(), 1024); }
            frame += 1;
            if cfg!(feature = "ci") && frame == 120 { exit(0) }
        }
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    unsafe { exit(1) }
}
