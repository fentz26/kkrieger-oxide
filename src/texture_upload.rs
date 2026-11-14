//! Original player texture conversion and 15-bit mip generation (sINTRO=1).
use crate::bitmap::{Bitmap, Pixel};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    Rgba8,
    Rgba8Snorm,
}
#[derive(Clone, Debug)]
pub struct Level {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}
#[derive(Clone, Debug)]
pub struct Pyramid {
    pub encoding: Encoding,
    /// D3D allocation count; non-square chains can contain unwritten tail levels.
    pub allocated_levels: usize,
    pub levels: Vec<Level>,
}
fn power2(value: usize) -> usize {
    usize::BITS as usize - 1 - value.leading_zeros() as usize
}
impl Pyramid {
    pub fn from_bitmap(bitmap: &Bitmap) -> Result<Self, &'static str> {
        let (mut width, mut height) = (bitmap.width, bitmap.height);
        if width == 0 || height == 0 || width.checked_mul(height) != Some(bitmap.pixels.len()) {
            return Err("invalid texture dimensions");
        }
        let encoding = match bitmap.format {
            1 => Encoding::Rgba8,
            5 => Encoding::Rgba8Snorm,
            _ => return Err("texture format has not been ported"),
        };
        let mut count = (bitmap.mip_count as usize)
            .min(width.next_power_of_two().trailing_zeros().max(1) as usize)
            .min(height.next_power_of_two().trailing_zeros().max(1) as usize);
        if !width.is_power_of_two() || !height.is_power_of_two() {
            count = 1;
        }
        let allocated_levels = if count == 0 {
            power2(width.max(height)) + 1
        } else {
            count
        };
        let mut result = Self {
            encoding,
            allocated_levels,
            levels: Vec::new(),
        };
        let mut source = bitmap.pixels.clone();
        let mut level = 0;
        loop {
            let pixels = source
                .iter()
                .flat_map(|p| {
                    let channels = [p[2], p[1], p[0], p[3]];
                    channels.map(|v| match encoding {
                        Encoding::Rgba8 => ((v >> 7) & 255) as u8,
                        Encoding::Rgba8Snorm => (((v as i32 - 0x4000) >> 7) & 255) as u8,
                    })
                })
                .collect();
            result.levels.push(Level {
                width,
                height,
                pixels,
            });
            if width <= 1 || height <= 1 || (count != 0 && level + 1 >= count) {
                break;
            }
            let mut filtered = (bitmap.mip_threshold & 15) as usize <= level;
            if bitmap.mip_threshold & 16 != 0 {
                filtered = !filtered;
            }
            let mut next: Vec<Pixel> = Vec::with_capacity(width / 2 * (height / 2));
            for y in 0..height / 2 {
                for x in 0..width / 2 {
                    let i = y * 2 * width + x * 2;
                    let p = if filtered {
                        std::array::from_fn(|c| {
                            ((source[i][c] as u32
                                + source[i + 1][c] as u32
                                + source[i + width][c] as u32
                                + source[i + width + 1][c] as u32
                                + 2)
                                >> 2) as u16
                        })
                    } else {
                        source[i]
                    };
                    next.push(p);
                }
            }
            source = next;
            width >>= 1;
            height >>= 1;
            level += 1;
        }
        Ok(result)
    }
}
