//! Original FontBegin/FontPrint GDI settings, with owned and restored objects.
use crate::bitmap::{Error, Result};
use std::{
    ffi::{CStr, CString, c_char, c_void},
    ptr,
};
type Handle = *mut c_void;
#[repr(C)]
#[derive(Default)]
struct Header {
    size: u32,
    width: i32,
    height: i32,
    planes: u16,
    bits: u16,
    compression: u32,
    image_size: u32,
    xppm: i32,
    yppm: i32,
    used: u32,
    important: u32,
}
#[repr(C)]
#[derive(Default)]
struct Info {
    header: Header,
    color: u32,
}
#[repr(C)]
#[derive(Default)]
struct Metrics {
    values: [i32; 11],
    bytes: [u8; 9],
}
#[repr(C)]
#[derive(Default)]
struct Abc {
    a: i32,
    b: u32,
    c: i32,
}
#[repr(C)]
#[derive(Default)]
struct Size {
    x: i32,
    y: i32,
}
const _: () = {
    assert!(size_of::<Header>() == 40);
    assert!(size_of::<Info>() == 44);
    assert!(size_of::<Metrics>() == 56);
    assert!(size_of::<Abc>() == 12);
};
#[link(name = "gdi32")]
unsafe extern "system" {
    fn CreateCompatibleDC(dc: Handle) -> Handle;
    fn DeleteDC(dc: Handle) -> i32;
    fn CreateDIBSection(
        dc: Handle,
        info: *const Info,
        usage: u32,
        bits: *mut *mut c_void,
        section: Handle,
        offset: u32,
    ) -> Handle;
    fn SelectObject(dc: Handle, object: Handle) -> Handle;
    fn DeleteObject(object: Handle) -> i32;
    fn CreateFontA(
        height: i32,
        width: i32,
        escape: i32,
        orientation: i32,
        weight: i32,
        italic: u32,
        underline: u32,
        strike: u32,
        charset: u32,
        out: u32,
        clip: u32,
        quality: u32,
        pitch: u32,
        name: *const c_char,
    ) -> Handle;
    fn SetBkMode(dc: Handle, mode: i32) -> i32;
    fn SetTextColor(dc: Handle, color: u32) -> u32;
    fn GetTextFaceA(dc: Handle, count: i32, name: *mut c_char) -> i32;
    fn GetTextMetricsA(dc: Handle, metrics: *mut Metrics) -> i32;
    fn GetCharABCWidthsA(dc: Handle, first: u32, last: u32, abc: *mut Abc) -> i32;
    fn GetTextExtentPoint32A(dc: Handle, text: *const c_char, len: i32, size: *mut Size) -> i32;
    fn ExtTextOutA(
        dc: Handle,
        x: i32,
        y: i32,
        options: u32,
        rect: *const c_void,
        text: *const c_char,
        len: u32,
        dx: *const i32,
    ) -> i32;
    fn GdiFlush() -> i32;
}
#[link(name = "user32")]
unsafe extern "system" {
    fn GetDC(window: Handle) -> Handle;
    fn ReleaseDC(window: Handle, dc: Handle) -> i32;
}
fn selected(dc: Handle, object: Handle) -> Result<Handle> {
    let old = unsafe { SelectObject(dc, object) };
    if old.is_null() || old as usize == usize::MAX {
        Err(Error("GDI object selection failed"))
    } else {
        Ok(old)
    }
}
pub(super) struct Raster {
    screen: Handle,
    memory: Handle,
    bitmap: Handle,
    font: Handle,
    old_bitmap: Handle,
    old_screen_font: Handle,
    old_memory_font: Handle,
    pixels: *mut u32,
    size: [usize; 2],
    pub line_height: i32,
}
impl Raster {
    pub fn new(name: &[u8], size: [usize; 2], vertical: i32, horizontal: i32) -> Result<Self> {
        let name = CString::new(if name.is_empty() {
            b"arial".as_slice()
        } else {
            name
        })
        .map_err(|_| Error("invalid GDI font name"))?;
        let mut r = Self {
            screen: ptr::null_mut(),
            memory: ptr::null_mut(),
            bitmap: ptr::null_mut(),
            font: ptr::null_mut(),
            old_bitmap: ptr::null_mut(),
            old_screen_font: ptr::null_mut(),
            old_memory_font: ptr::null_mut(),
            pixels: ptr::null_mut(),
            size,
            line_height: 0,
        };
        r.screen = unsafe { GetDC(ptr::null_mut()) };
        if r.screen.is_null() {
            return Err(Error("GDI screen context creation failed"));
        }
        r.memory = unsafe { CreateCompatibleDC(r.screen) };
        if r.memory.is_null() {
            return Err(Error("GDI memory context creation failed"));
        }
        let info = Info {
            header: Header {
                size: 40,
                width: i32::try_from(size[0]).map_err(|_| Error("GDI canvas width overflow"))?,
                height: -i32::try_from(size[1]).map_err(|_| Error("GDI canvas height overflow"))?,
                planes: 1,
                bits: 32,
                ..Header::default()
            },
            color: 0,
        };
        let mut pixels = ptr::null_mut();
        r.bitmap = unsafe { CreateDIBSection(r.screen, &info, 0, &mut pixels, ptr::null_mut(), 0) };
        r.pixels = pixels.cast();
        if r.bitmap.is_null() || r.pixels.is_null() {
            return Err(Error("GDI bitmap creation failed"));
        }
        let pixels = size[0]
            .checked_mul(size[1])
            .filter(|&count| count <= isize::MAX as usize / size_of::<u32>())
            .ok_or(Error("GDI canvas overflow"))?;
        // Define the background before any native drawing or Rust pixel reads.
        unsafe {
            ptr::write_bytes(r.pixels, 0, pixels);
        }
        r.old_bitmap = selected(r.memory, r.bitmap)?;
        let font_height = vertical
            .checked_neg()
            .ok_or(Error("GDI font height overflow"))?;
        // Original style 4: normal weight, DEFAULT_CHARSET, OUT_TT_PRECIS,
        // CLIP_DEFAULT_PRECIS, PROOF_QUALITY, DEFAULT_PITCH|FF_DONTCARE.
        r.font = unsafe {
            CreateFontA(
                font_height,
                horizontal,
                0,
                0,
                400,
                0,
                0,
                0,
                1,
                4,
                0,
                2,
                0,
                name.as_ptr(),
            )
        };
        if r.font.is_null() {
            return Err(Error("GDI font creation failed"));
        }
        r.old_screen_font = selected(r.screen, r.font)?;
        r.old_memory_font = selected(r.memory, r.font)?;
        if unsafe { SetBkMode(r.memory, 1) } == 0
            || unsafe { SetTextColor(r.memory, 0xffffff) } == u32::MAX
        {
            return Err(Error("GDI text state setup failed"));
        }
        let mut face = [0u8; 128];
        if unsafe { GetTextFaceA(r.memory, face.len() as i32, face.as_mut_ptr().cast()) } == 0 {
            return Err(Error("GDI font identification failed"));
        }
        let face = CStr::from_bytes_until_nul(&face).map_err(|_| Error("invalid GDI font name"))?;
        if !super::font_face_matches(name.as_bytes(), face.to_bytes()) {
            return Err(Error("original requested font is not installed"));
        }
        let mut met = Metrics::default();
        if unsafe { GetTextMetricsA(r.memory, &mut met) } == 0 {
            return Err(Error("GDI font metrics failed"));
        }
        r.line_height = met.values[0];
        if r.line_height <= 0 {
            return Err(Error("invalid GDI font height"));
        }
        Ok(r)
    }
    pub fn metrics(&self, byte: u8) -> Result<[i32; 3]> {
        let mut abc = Abc::default();
        if unsafe { GetCharABCWidthsA(self.memory, u32::from(byte), u32::from(byte), &mut abc) }
            == 0
        {
            return Err(Error("GDI glyph metrics failed"));
        }
        Ok([abc.a, abc.b as i32, abc.c])
    }
    pub fn width(&self, text: &[u8]) -> Result<i32> {
        let len = i32::try_from(text.len()).map_err(|_| Error("GDI text length overflow"))?;
        let mut size = Size::default();
        if unsafe { GetTextExtentPoint32A(self.memory, text.as_ptr().cast(), len, &mut size) } == 0
        {
            return Err(Error("GDI text measurement failed"));
        }
        Ok(size.x)
    }
    pub fn print(&mut self, position: [i32; 2], text: &[u8]) -> Result<()> {
        if text.is_empty() {
            return Ok(());
        }
        let len = u32::try_from(text.len()).map_err(|_| Error("GDI text length overflow"))?;
        if unsafe {
            ExtTextOutA(
                self.memory,
                position[0],
                position[1],
                0,
                ptr::null(),
                text.as_ptr().cast(),
                len,
                ptr::null(),
            )
        } == 0
        {
            return Err(Error("GDI text drawing failed"));
        }
        Ok(())
    }
    pub fn finish(self) -> Result<Vec<u8>> {
        // Required synchronization before accessing CreateDIBSection memory.
        if unsafe { GdiFlush() } == 0 {
            return Err(Error("GDI text flush failed"));
        }
        let count = self.size[0]
            .checked_mul(self.size[1])
            .ok_or(Error("GDI canvas overflow"))?;
        let pixels = unsafe { std::slice::from_raw_parts(self.pixels, count) };
        Ok(pixels.iter().map(|&pixel| pixel as u8).collect())
    }
}
impl Drop for Raster {
    fn drop(&mut self) {
        unsafe {
            GdiFlush();
            if !self.old_memory_font.is_null() {
                SelectObject(self.memory, self.old_memory_font);
            }
            if !self.old_screen_font.is_null() {
                SelectObject(self.screen, self.old_screen_font);
            }
            if !self.old_bitmap.is_null() {
                SelectObject(self.memory, self.old_bitmap);
            }
            if !self.font.is_null() {
                DeleteObject(self.font);
            }
            if !self.bitmap.is_null() {
                DeleteObject(self.bitmap);
            }
            if !self.memory.is_null() {
                DeleteDC(self.memory);
            }
            if !self.screen.is_null() {
                ReleaseDC(ptr::null_mut(), self.screen);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unavailable_face_is_rejected_instead_of_using_gdi_fallback() {
        let result = Raster::new(b"KkriegerMissingFont123", [64, 64], 16, 0);
        assert!(matches!(
            result,
            Err(Error("original requested font is not installed"))
        ));
    }
}
