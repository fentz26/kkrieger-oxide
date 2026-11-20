//! Size probe: the platform services kkrieger needs on macOS, called directly
//! (Cocoa window/events, Metal clear+present, AudioQueue PCM16, CoreText glyphs),
//! with no std. Objective-C messages go through objc_msgSend with exact types.
#![no_std]
#![no_main]
use core::{ffi::c_void, ptr::null_mut};
type Id = *mut c_void;
type Sel = *const c_void;

#[link(name = "System")]
unsafe extern "C" {
    fn exit(code: i32) -> !;
    fn mach_absolute_time() -> u64;
}
#[link(name = "objc")]
unsafe extern "C" {
    fn objc_getClass(name: *const u8) -> Id;
    fn sel_registerName(name: *const u8) -> Sel;
    fn objc_msgSend();
}
#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {}
#[link(name = "QuartzCore", kind = "framework")]
unsafe extern "C" {}
#[link(name = "Metal", kind = "framework")]
unsafe extern "C" {
    fn MTLCreateSystemDefaultDevice() -> Id;
}
#[link(name = "AudioToolbox", kind = "framework")]
unsafe extern "C" {
    fn AudioQueueNewOutput(fmt: *const Asbd, cb: extern "C" fn(*mut c_void, Id, *mut AqBuffer), user: *mut c_void, rl: Id, mode: Id, flags: u32, out: *mut Id) -> i32;
    fn AudioQueueAllocateBuffer(q: Id, size: u32, out: *mut *mut AqBuffer) -> i32;
    fn AudioQueueEnqueueBuffer(q: Id, b: *mut AqBuffer, n: u32, d: *const c_void) -> i32;
    fn AudioQueueStart(q: Id, t: *const c_void) -> i32;
}
#[link(name = "CoreText", kind = "framework")]
unsafe extern "C" {
    fn CTFontCreateWithName(name: Id, size: f64, m: *const c_void) -> Id;
    fn CTFontGetGlyphsForCharacters(f: Id, c: *const u16, g: *mut u16, n: isize) -> bool;
    fn CTFontGetAdvancesForGlyphs(f: Id, o: u32, g: *const u16, a: *mut [f64; 2], n: isize) -> f64;
    fn CTFontDrawGlyphs(f: Id, g: *const u16, p: *const [f64; 2], n: usize, ctx: Id);
}
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGColorSpaceCreateDeviceGray() -> Id;
    fn CGBitmapContextCreate(d: *mut u8, w: usize, h: usize, bpc: usize, bpr: usize, cs: Id, info: u32) -> Id;
}
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFStringCreateWithCString(a: Id, s: *const u8, enc: u32) -> Id;
}
#[repr(C)]
struct Asbd { rate: f64, format: u32, flags: u32, bpp: u32, fpp: u32, bpf: u32, ch: u32, bits: u32, res: u32 }
#[repr(C)]
struct AqBuffer { cap: u32, data: *mut i16, size: u32 }

macro_rules! msg {
    ($o:expr, $s:literal $(, $a:expr => $t:ty)* ; $r:ty) => {{
        let f: unsafe extern "C" fn(Id, Sel $(, $t)*) -> $r = core::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        f($o, sel_registerName(concat!($s, "\0").as_ptr()) $(, $a)*)
    }};
}
fn class(n: &[u8]) -> Id { unsafe { objc_getClass(n.as_ptr()) } }

extern "C" fn audio(_: *mut c_void, q: Id, b: *mut AqBuffer) {
    unsafe { (*b).size = (*b).cap; AudioQueueEnqueueBuffer(q, b, 0, null_mut()); }
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    unsafe {
        let app: Id = msg!(class(b"NSApplication\0"), "sharedApplication"; Id);
        msg!(app, "setActivationPolicy:", 0isize => isize; bool);
        let window: Id = msg!(class(b"NSWindow\0"), "alloc"; Id);
        let window: Id = msg!(window, "initWithContentRect:styleMask:backing:defer:",
            [0f64, 0., 800., 600.] => [f64; 4], 15usize => usize, 2usize => usize, false => bool; Id);
        msg!(window, "makeKeyAndOrderFront:", null_mut() => Id; ());
        let view: Id = msg!(window, "contentView"; Id);
        msg!(view, "setWantsLayer:", true => bool; ());
        let layer: Id = msg!(class(b"CAMetalLayer\0"), "layer"; Id);
        let device = MTLCreateSystemDefaultDevice();
        msg!(layer, "setDevice:", device => Id; ());
        msg!(view, "setLayer:", layer => Id; ());
        let queue: Id = msg!(device, "newCommandQueue"; Id);
        msg!(app, "activateIgnoringOtherApps:", true => bool; ());
        // CoreText glyph rasterization for the original Arial/Times atlases.
        let name = CFStringCreateWithCString(null_mut(), b"Arial\0".as_ptr(), 0x0800_0100);
        let font = CTFontCreateWithName(name, 32., null_mut());
        static mut PIXELS: [u8; 256 * 256] = [0; 256 * 256];
        let ctx = CGBitmapContextCreate(core::ptr::addr_of_mut!(PIXELS).cast(), 256, 256, 8, 256, CGColorSpaceCreateDeviceGray(), 0);
        let chars: [u16; 8] = [107, 107, 114, 105, 101, 103, 101, 114];
        let mut glyphs = [0u16; 8];
        let mut advances = [[0f64; 2]; 8];
        CTFontGetGlyphsForCharacters(font, chars.as_ptr(), glyphs.as_mut_ptr(), 8);
        CTFontGetAdvancesForGlyphs(font, 0, glyphs.as_ptr(), advances.as_mut_ptr(), 8);
        CTFontDrawGlyphs(font, glyphs.as_ptr(), advances.as_ptr(), 8, ctx);
        // AudioQueue at the original 44.1 kHz stereo PCM16.
        let fmt = Asbd { rate: 44100., format: u32::from_be_bytes(*b"lpcm"), flags: 12, bpp: 4, fpp: 1, bpf: 4, ch: 2, bits: 16, res: 0 };
        let mut aq = null_mut();
        AudioQueueNewOutput(&fmt, audio, null_mut(), null_mut(), null_mut(), 0, &mut aq);
        for _ in 0..3 {
            let mut b = null_mut();
            AudioQueueAllocateBuffer(aq, 16384, &mut b);
            audio(null_mut(), aq, b);
        }
        AudioQueueStart(aq, null_mut());
        let mode: Id = msg!(class(b"NSString\0"), "stringWithUTF8String:", b"kCFRunLoopDefaultMode\0".as_ptr() => *const u8; Id);
        let start = mach_absolute_time();
        let mut presented = 0u32;
        loop {
            loop {
                let event: Id = msg!(app, "nextEventMatchingMask:untilDate:inMode:dequeue:",
                    u64::MAX => u64, null_mut() => Id, mode => Id, true => bool; Id);
                if event.is_null() { break; }
                let kind: usize = msg!(event, "type"; usize);
                if kind == 10 && msg!(event, "keyCode"; u16) == 53 { exit(0) }
                let _: [f64; 2] = msg!(event, "deltaX"; [f64; 2]);
                msg!(app, "sendEvent:", event => Id; ());
            }
            let t = (mach_absolute_time() - start) as f64 * 1e-9;
            let drawable: Id = msg!(layer, "nextDrawable"; Id);
            if drawable.is_null() { continue; }
            let pass: Id = msg!(class(b"MTLRenderPassDescriptor\0"), "renderPassDescriptor"; Id);
            let attachments: Id = msg!(pass, "colorAttachments"; Id);
            let color: Id = msg!(attachments, "objectAtIndexedSubscript:", 0usize => usize; Id);
            let texture: Id = msg!(drawable, "texture"; Id);
            msg!(color, "setTexture:", texture => Id; ());
            msg!(color, "setLoadAction:", 2usize => usize; ());
            msg!(color, "setClearColor:", [t % 1., 0., 0., 1.] => [f64; 4]; ());
            let buffer: Id = msg!(queue, "commandBuffer"; Id);
            let encoder: Id = msg!(buffer, "renderCommandEncoderWithDescriptor:", pass => Id; Id);
            msg!(encoder, "endEncoding"; ());
            msg!(buffer, "presentDrawable:", drawable => Id; ());
            msg!(buffer, "commit"; ());
            presented += 1;
            if cfg!(feature = "ci") && presented == 120 { exit(0) }
        }
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    unsafe { exit(1) }
}
