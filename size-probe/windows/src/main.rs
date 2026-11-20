//! Size probe: the platform services kkrieger needs on Windows, called
//! directly (Win32 window/input, D3D9, WinMM audio, GDI text), with no std.
#![no_std]
#![no_main]
use core::{ffi::c_void, ptr::null_mut};
type H = *mut c_void;

#[link(name = "kernel32", kind = "raw-dylib")]
unsafe extern "system" {
    fn GetModuleHandleA(n: *const u8) -> H;
    fn ExitProcess(code: u32) -> !;
    fn Sleep(ms: u32);
}
#[link(name = "user32", kind = "raw-dylib")]
unsafe extern "system" {
    fn RegisterClassA(c: *const WndClass) -> u16;
    fn CreateWindowExA(ex: u32, class: *const u8, title: *const u8, style: u32, x: i32, y: i32, w: i32, h: i32, parent: H, menu: H, inst: H, param: H) -> H;
    fn PeekMessageA(m: *mut Msg, w: H, a: u32, b: u32, remove: u32) -> i32;
    fn DispatchMessageA(m: *const Msg) -> isize;
    fn DefWindowProcA(w: H, m: u32, wp: usize, lp: isize) -> isize;
    fn GetAsyncKeyState(key: i32) -> i16;
    fn GetCursorPos(p: *mut [i32; 2]) -> i32;
    fn SetCursorPos(x: i32, y: i32) -> i32;
    fn ShowCursor(show: i32) -> i32;
}
#[link(name = "gdi32", kind = "raw-dylib")]
unsafe extern "system" {
    fn CreateCompatibleDC(dc: H) -> H;
    fn CreateFontA(h: i32, w: i32, e: i32, o: i32, wt: i32, i: u32, u: u32, s: u32, cs: u32, op: u32, cp: u32, q: u32, pf: u32, face: *const u8) -> H;
    fn SelectObject(dc: H, o: H) -> H;
    fn ExtTextOutA(dc: H, x: i32, y: i32, o: u32, r: *const c_void, s: *const u8, n: u32, dx: *const i32) -> i32;
    fn CreateDIBSection(dc: H, bmi: *const [u32; 11], usage: u32, bits: *mut *mut u32, sec: H, off: u32) -> H;
    fn GetCharABCWidthsA(dc: H, first: u32, last: u32, abc: *mut [i32; 3]) -> i32;
}
#[link(name = "winmm", kind = "raw-dylib")]
unsafe extern "system" {
    fn waveOutOpen(h: *mut H, dev: u32, fmt: *const [u16; 9], cb: usize, inst: usize, flags: u32) -> u32;
    fn waveOutPrepareHeader(h: H, hdr: *mut WaveHdr, size: u32) -> u32;
    fn waveOutWrite(h: H, hdr: *mut WaveHdr, size: u32) -> u32;
    fn timeGetTime() -> u32;
}
#[link(name = "d3d9", kind = "raw-dylib")]
unsafe extern "system" {
    fn Direct3DCreate9(version: u32) -> *mut *const usize;
}
#[repr(C)]
struct WndClass { style: u32, proc_: unsafe extern "system" fn(H, u32, usize, isize) -> isize, a: i32, b: i32, inst: H, icon: H, cursor: H, brush: H, menu: *const u8, name: *const u8 }
#[repr(C)]
struct Msg { w: H, m: u32, wp: usize, lp: isize, t: u32, p: [i32; 2] }
#[repr(C)]
struct WaveHdr { data: *mut i16, len: u32, rec: u32, user: usize, flags: u32, loops: u32, next: H, res: usize }

static mut AUDIO: [i16; 2 * 4096] = [0; 2 * 4096];
#[unsafe(no_mangle)]
static _fltused: i32 = 0;

unsafe extern "system" fn proc_(w: H, m: u32, wp: usize, lp: isize) -> isize {
    if m == 0x10 { unsafe { ExitProcess(0) } }
    unsafe { DefWindowProcA(w, m, wp, lp) }
}

// COM call through vtable slot n.
macro_rules! com { ($o:expr, $n:expr, $t:ty $(, $a:expr)*) => {{
    let o = $o; let f: $t = core::mem::transmute(*(*o).add($n)); f(o $(, $a)*)
}}; }

#[unsafe(no_mangle)]
pub unsafe extern "system" fn mainCRTStartup() -> ! {
    unsafe {
        let inst = GetModuleHandleA(null_mut());
        let class = b"kk\0".as_ptr();
        RegisterClassA(&WndClass { style: 0, proc_, a: 0, b: 0, inst, icon: null_mut(), cursor: null_mut(), brush: null_mut(), menu: null_mut(), name: class });
        let w = CreateWindowExA(0, class, class, 0x10cf0000, 0, 0, 800, 600, null_mut(), null_mut(), inst, null_mut());
        // GDI text path used by the original font generator.
        let dc = CreateCompatibleDC(null_mut());
        let bmi = [44u32, 256, (-256i32) as u32, 0x0020_0001, 0, 0, 0, 0, 0, 0, 0];
        let mut bits = null_mut();
        SelectObject(dc, CreateDIBSection(dc, &bmi, 0, &mut bits, null_mut(), 0));
        SelectObject(dc, CreateFontA(32, 0, 0, 0, 400, 0, 0, 0, 0, 0, 0, 2, 0, b"Arial\0".as_ptr()));
        let mut abc = [[0i32; 3]; 256];
        GetCharABCWidthsA(dc, 0, 255, abc.as_mut_ptr());
        ExtTextOutA(dc, 0, 0, 0, null_mut(), b"kkrieger".as_ptr(), 8, null_mut());
        // WinMM 44.1 kHz stereo PCM16, as the original.
        let mut wave = null_mut();
        waveOutOpen(&mut wave, u32::MAX, &[1, 2, 44100, 0, 0xb110, 2, 4, 16, 0], 0, 0, 0);
        let mut hdr = WaveHdr { data: core::ptr::addr_of_mut!(AUDIO).cast(), len: 4 * 4096, rec: 0, user: 0, flags: 0, loops: 0, next: null_mut(), res: 0 };
        waveOutPrepareHeader(wave, &mut hdr, 32);
        waveOutWrite(wave, &mut hdr, 32);
        // D3D9 device: create, then clear and present each frame.
        let d3d = Direct3DCreate9(32);
        let mut pp = [0u32; 14];
        pp[6] = 1; // discard
        pp[7] = w as usize as u32;
        pp[8] = 1; // windowed
        let mut device: *mut *const usize = null_mut();
        com!(d3d, 16, unsafe extern "system" fn(*mut *const usize, u32, u32, H, u32, *mut [u32; 14], *mut *mut *const usize) -> i32, 0, 1, w, 0x20, &mut pp, &mut device);
        ShowCursor(0);
        let start = timeGetTime();
        let mut msg = core::mem::zeroed::<Msg>();
        let mut frame = 0u32;
        loop {
            frame += 1;
            if cfg!(feature = "ci") && frame == 120 {
                ExitProcess(if device.is_null() { 2 } else { 0 })
            }
            while PeekMessageA(&mut msg, null_mut(), 0, 0, 1) != 0 { DispatchMessageA(&msg); }
            if GetAsyncKeyState(0x1b) < 0 { ExitProcess(0) }
            let mut cursor = [0; 2];
            GetCursorPos(&mut cursor);
            SetCursorPos(400, 300);
            let t = timeGetTime().wrapping_sub(start);
            if !device.is_null() {
                com!(device, 41, unsafe extern "system" fn(*mut *const usize) -> i32);
                com!(device, 43, unsafe extern "system" fn(*mut *const usize, u32, *const c_void, u32, u32, f32, u32) -> i32, 0, null_mut(), 3, t & 0xff, 1.0, 0);
                com!(device, 42, unsafe extern "system" fn(*mut *const usize) -> i32);
                com!(device, 17, unsafe extern "system" fn(*mut *const usize, *const c_void, *const c_void, H, *const c_void) -> i32, null_mut(), null_mut(), null_mut(), null_mut());
            }
            Sleep(1);
        }
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    unsafe { ExitProcess(1) }
}
