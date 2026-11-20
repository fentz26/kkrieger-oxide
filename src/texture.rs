//! Evaluate original texture graph nodes. Unsupported nodes fail explicitly;
//! they never become placeholder textures. Consumed intermediates are released.
use crate::{
    bitmap::{self, Bitmap, Bump, Cell, FontBook, GlowRect, Text, Transform, Twirl},
    kx::Document,
    random::Noise,
};
use std::{collections::BTreeMap, fmt, rc::Rc};

#[derive(Default)]
pub struct Generated {
    pub images: Vec<Rc<Bitmap>>,
    /// Original Bitmap_Text writes KEnvironment::Letters during precalculation,
    /// independently of whether its bitmap survives as an output texture.
    pub font_pages: BTreeMap<u8, bitmap::FontPage>,
}

#[derive(Debug)]
pub struct Error {
    pub operator: usize,
    pub class: u16,
    pub reason: &'static str,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "texture op {} (0x{:02x}): {}",
            self.operator, self.class, self.reason
        )
    }
}
impl std::error::Error for Error {}

pub fn implemented(id: u16) -> bool {
    matches!(id, 0x21..=0x25 | 0x27..=0x2e | 0x30..=0x32 | 0x34..=0x39)
}

pub fn generate(doc: &Document<'_>, root: usize, size_offset: i32) -> Result<Bitmap, Error> {
    let mut images = generate_many(doc, &[root], size_offset)?;
    let image = images.pop().ok_or(Error {
        operator: root,
        class: 0,
        reason: "empty texture batch",
    })?;
    Rc::try_unwrap(image).map_err(|_| Error {
        operator: root,
        class: 0,
        reason: "unexpected retained intermediate",
    })
}

/// Evaluate shared dependencies once and release non-output intermediates at
/// their final consumer. Duplicate roots share the same generated allocation.
pub fn generate_many(
    doc: &Document<'_>,
    roots: &[usize],
    size_offset: i32,
) -> Result<Vec<Rc<Bitmap>>, Error> {
    Ok(generate_many_with_fonts(doc, roots, size_offset)?.images)
}

/// Retain font-atlas environment side effects as well as the requested images.
pub fn generate_many_with_fonts(
    doc: &Document<'_>,
    roots: &[usize],
    size_offset: i32,
) -> Result<Generated, Error> {
    let error = |i: usize, reason| Error {
        operator: i,
        class: doc
            .operators
            .get(i)
            .map(|o| doc.classes[o.class].id)
            .unwrap_or(0),
        reason,
    };
    if roots.is_empty() {
        return Ok(Generated::default());
    }
    for &root in roots {
        if root >= doc.operators.len() {
            return Err(error(root, "root out of range"));
        }
    }
    let mut needed = vec![false; doc.operators.len()];
    let mut pending = roots.to_vec();
    while let Some(i) = pending.pop() {
        if needed[i] {
            continue;
        }
        let op = &doc.operators[i];
        let id = doc.classes[op.class].id;
        if !implemented(id) {
            return Err(error(i, "operator has not been ported"));
        }
        if op.animation != [1] {
            return Err(error(
                i,
                "animated texture requires an evaluation environment",
            ));
        }
        if !op.links.is_empty() {
            return Err(error(i, "linked texture inputs are not implemented"));
        }
        needed[i] = true;
        pending.extend(op.inputs.iter().copied());
    }
    let mut users = vec![0_usize; doc.operators.len()];
    for &root in roots {
        users[root] += 1;
    }
    for (i, op) in doc.operators.iter().enumerate().filter(|(i, _)| needed[*i]) {
        for &input in &op.inputs {
            if input >= i {
                return Err(error(i, "input is not topologically ordered"));
            }
            users[input] += 1;
        }
    }
    let mut cache: Vec<Option<Rc<Bitmap>>> = (0..doc.operators.len()).map(|_| None).collect();
    let noise = Noise::new();
    let mut fonts = FontBook::new();
    let mut font_pages = BTreeMap::new();
    let last = *roots.iter().max().unwrap();
    for i in 0..=last {
        if !needed[i] {
            continue;
        }
        let op = &doc.operators[i];
        let id = doc.classes[op.class].id;
        let mut inputs = Vec::with_capacity(op.inputs.len());
        for &input in &op.inputs {
            inputs.push(
                cache[input]
                    .as_ref()
                    .ok_or(error(i, "input bitmap unavailable"))?
                    .clone(),
            );
            users[input] -= 1;
            if users[input] == 0 {
                cache[input] = None;
            }
        }
        let mut first = if inputs.is_empty() {
            None
        } else {
            Some(inputs.remove(0))
        };
        let int = |n| {
            op.parameters
                .get(n)
                .copied()
                .map(|v| v as i32)
                .ok_or(error(i, "missing integer parameter"))
        };
        let float = |n| op.float(n).ok_or(error(i, "missing float parameter"));
        let mut with_first=|f:&mut dyn FnMut(&mut Bitmap)->Result<(),bitmap::Error>|->Result<Rc<Bitmap>,Error> {
            let mut bitmap=first.take().ok_or(error(i,"missing input bitmap"))?;
            f(Rc::make_mut(&mut bitmap)).map_err(|e|error(i,e.0))?;
            Ok(bitmap)
        };
        let mut result = match id {
            0x21 => Rc::new(Bitmap::flat(int(0)?, int(1)?, int(2)? as u32, size_offset)),
            0x22 => Rc::new(
                Bitmap::perlin(
                    int(0)?,
                    int(1)?,
                    int(2)?,
                    int(3)?,
                    float(4)?,
                    int(5)? as u8,
                    int(6)? as u32,
                    float(7)?,
                    float(8)?,
                    int(9)? as u32,
                    int(10)? as u32,
                    size_offset,
                    &noise,
                )
                .map_err(|e| error(i, e.0))?,
            ),
            0x23 => {
                let (mode, color) = (int(0)? as u32, int(1)? as u32);
                with_first(&mut |b| b.color(mode, color))?
            }
            0x24 => {
                let mode = int(0)? as u32;
                with_first(&mut |b| {
                    for other in &inputs {
                        b.merge(other, mode)?;
                    }
                    Ok(())
                })?
            }
            0x25 => {
                let (format, count, threshold) = (int(0)?, int(1)?, int(2)?);
                if format <= 0 {
                    return Err(error(i, "invalid texture format"));
                }
                with_first(&mut |b| {
                    b.format = format as u32;
                    b.mip_count = count as u32;
                    b.mip_threshold = threshold as u32;
                    Ok(())
                })?
            }
            0x27 | 0x39 => {
                let p = GlowRect {
                    center: [float(0)?, float(1)?],
                    radius: [float(2)?, float(3)?],
                    solid: [float(4)?, float(5)?],
                    color: int(6)? as u32,
                    alpha: float(7)?,
                    power: float(8)?,
                    wrap: int(9)? as u32,
                    legacy: int(10)? != 0,
                };
                with_first(&mut |b| {
                    b.glow(p);
                    Ok(())
                })?
            }
            0x29 => {
                let (flags, sx, sy, amp) = (int(0)? as u32, float(1)?, float(2)?, float(3)?);
                with_first(&mut |b| b.blur(flags, sx, sy, amp))?
            }
            0x28 => {
                let (a, b, density, seed) = (int(0)?, int(1)?, int(2)?, int(3)?);
                with_first(&mut |bm| bm.dots(a as u32, b as u32, density, seed))?
            }
            0x2a => {
                if inputs.len() != 2 {
                    return Err(error(i, "mask requires three images"));
                }
                let mode = int(0)? as u32;
                with_first(&mut |b| b.mask(&inputs[0], &inputs[1], mode))?
            }
            0x2b => {
                let (h, s, c, b) = (float(0)?, float(1)?, float(2)?, float(3)?);
                with_first(&mut |bm| {
                    bm.hscb(h, s, c, b);
                    Ok(())
                })?
            }
            0x2c => {
                let p = Transform {
                    angle: float(0)?,
                    scale: [float(1)?, float(2)?],
                    translate: [float(3)?, float(4)?],
                    border: int(5)? as u32,
                    new_size: [op.integer(6).unwrap_or(0), op.integer(7).unwrap_or(0)],
                };
                with_first(&mut |b| b.rotate(p, size_offset))?
            }
            0x2d => {
                if inputs.len() != 1 {
                    return Err(error(i, "distortion requires two images"));
                }
                let (distance, border) = (float(0)?, int(1)? as u32);
                with_first(&mut |b| {
                    let mut field = (*inputs[0]).clone();
                    field.distort(b, distance, border)?;
                    *b = field;
                    Ok(())
                })?
            }
            0x2e => {
                let (distance, mode) = (float(0)?, int(1)? as u32);
                with_first(&mut |b| {
                    b.normals(distance, mode);
                    Ok(())
                })?
            }
            0x30 => {
                let p = Bump {
                    kind: int(0)? as u32,
                    position: [float(1)?, float(2)?, float(3)?],
                    direction: [float(4)?, float(5)?],
                    diffuse: int(6)? as u32,
                    ambient: int(7)? as u32,
                    outer: float(8)?,
                    falloff: float(9)?,
                    amplitude: float(10)?,
                    specular: int(11)? as u32,
                    specular_power: float(12)?,
                    specular_amplitude: float(13)?,
                };
                with_first(&mut |b| b.bump(inputs.first().map(|b| &**b), p))?
            }
            0x31 => {
                let text = op
                    .strings
                    .first()
                    .ok_or(error(i, "missing original text"))?;
                let font = op
                    .strings
                    .get(1)
                    .ok_or(error(i, "missing original font name"))?;
                let p = Text {
                    position: [float(0)?, float(1)?],
                    width: float(2)?,
                    height: float(3)?,
                    color: int(4)? as u32,
                    flags: int(5)? as u32,
                    external_space: int(6)?,
                    internal_space: op.float(7).unwrap_or(0.),
                    line_skip: op.float(8).unwrap_or(1.),
                    text,
                    font,
                    blob: op.blob,
                };
                with_first(&mut |b| b.text_for_layout(p, &mut fonts, doc.layout))?
            }
            0x32 => Rc::new(
                Bitmap::cell(
                    int(0)?,
                    int(1)?,
                    Cell {
                        colors: [int(2)? as u32, int(3)? as u32, int(4)? as u32],
                        count: int(5)? as usize,
                        seed: int(6)?,
                        amplitude: float(7)?,
                        gamma: float(8)?,
                        mode: int(9)? as u32,
                        min_distance: float(10)?,
                        percent: int(11)?,
                        aspect: float(12)?,
                    },
                    size_offset,
                )
                .map_err(|e| error(i, e.0))?,
            ),
            0x36 => {
                let t = Transform {
                    angle: float(0)?,
                    scale: [float(1)?, float(2)?],
                    translate: [float(3)?, float(4)?],
                    border: int(5)? as u32,
                    new_size: [0; 2],
                };
                let (color, mode, count, fade) = (
                    int(6)? as u32,
                    int(7)? as u32,
                    int(8)? as u32,
                    int(9)? as u32,
                );
                with_first(&mut |b| b.rotate_mul(t, color, mode, count, fade, size_offset))?
            }
            0x37 => {
                let p = Twirl {
                    strength: float(0)?,
                    gamma: float(1)?,
                    radius: [float(2)?, float(3)?],
                    center: [float(4)?, float(5)?],
                    border: int(6)? as u32,
                };
                with_first(&mut |b| b.twirl(p))?
            }
            0x38 => {
                let (order, sx, sy, amp) = (int(0)? as u32, float(1)?, float(2)?, float(3)?);
                with_first(&mut |b| b.sharpen(order, sx, sy, amp))?
            }
            0x34 => Rc::new(
                Bitmap::gradient(
                    int(0)?,
                    int(1)?,
                    int(2)? as u32,
                    int(3)? as u32,
                    float(4)?,
                    float(5)?,
                    float(6)?,
                    int(7)? as u32,
                    size_offset,
                )
                .map_err(|e| error(i, e.0))?,
            ),
            0x35 => {
                let (mode, a, b) = (int(0)?, int(1)?, int(2)?);
                with_first(&mut |bm| bm.range(mode as u32, a as u32, b as u32))?
            }
            _ => return Err(error(i, "operator has not been ported")),
        };
        if id == 0x31
            && op.parameters.get(5).is_some_and(|flags| flags & 0x70 != 0)
            && let Some(page) = &result.font_page
        {
            font_pages.insert(page.page, (**page).clone());
        }
        // Every KX call retains the cached input and adds a call reference.
        // CheckBitmap therefore copies/allocates at operator boundaries.
        // GenBitmap::Copy copies pixels only: constructor defaults reset
        // format/mips. Only Format sets non-default texture metadata.
        if id != 0x25 {
            let image = Rc::make_mut(&mut result);
            image.format = 1;
            image.mip_count = 0;
            image.mip_threshold = 0;
        }
        cache[i] = Some(result);
    }
    let images = roots
        .iter()
        .map(|&root| {
            cache[root]
                .as_ref()
                .cloned()
                .ok_or(error(root, "empty evaluation result"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Generated { images, font_pages })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_perlin_assets_generate_without_placeholder_data() {
        let doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
        let mut generated = 0;
        let mut varied = false;
        for (i, op) in doc.operators.iter().enumerate() {
            if doc.classes[op.class].id != 0x22 {
                continue;
            }
            let bm = generate(&doc, i, -5).unwrap();
            assert_eq!(bm.pixels.len(), bm.width * bm.height);
            assert!(bm.pixels.iter().flatten().all(|&v| v <= 32767));
            varied |= bm.pixels.iter().any(|p| p != &bm.pixels[0]);
            generated += 1;
        }
        assert_eq!(generated, 31);
        assert!(varied);
    }
    #[test]
    fn unsupported_original_operators_are_reported() {
        let doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
        let i = doc
            .operators
            .iter()
            .position(|op| doc.classes[op.class].id == 0xc0)
            .unwrap();
        let error = generate(&doc, i, -5).unwrap_err();
        assert_eq!(error.class, 0xc0);
    }
    #[test]
    fn batched_branches_preserve_original_inputs_and_share_duplicate_roots() {
        let doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
        let batch = generate_many(&doc, &[3, 4, 3], -5).unwrap();
        assert!(Rc::ptr_eq(&batch[0], &batch[2]));
        for (image, index) in batch.iter().zip([3, 4, 3]) {
            let single = generate(&doc, index, -5).unwrap();
            assert_eq!(image.pixels, single.pixels);
        }
    }
    #[test]
    fn every_exported_texture_class_has_a_rust_handler() {
        let doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
        for c in &doc.classes {
            if (0x21..=0x39).contains(&c.id) {
                assert!(implemented(c.id), "class {:x}", c.id);
            }
        }
    }
    #[test]
    fn original_operator_copy_resets_format_and_mip_metadata() {
        let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
        let class = |id| doc.classes.iter().position(|c| c.id == id).unwrap();
        let ids = [class(0x21), class(0x25), class(0x23)];
        for (i, op) in doc.operators[..3].iter_mut().enumerate() {
            op.class = ids[i];
            op.inputs = if i == 0 { vec![] } else { vec![i - 1] };
            op.links.clear();
            op.animation = &[1];
        }
        doc.operators[0].parameters = vec![2, 2, 0xff808080];
        doc.operators[1].parameters = vec![2, 3, 17];
        doc.operators[2].parameters = vec![0, 0xffffffff];
        let b = generate_many(&doc, &[1, 2], 0).unwrap();
        assert_eq!(
            (b[0].format, b[0].mip_count, b[0].mip_threshold),
            (2, 3, 17)
        );
        assert_eq!((b[1].format, b[1].mip_count, b[1].mip_threshold), (1, 0, 0));
    }
}
