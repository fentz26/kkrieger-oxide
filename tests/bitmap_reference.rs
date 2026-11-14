use kkrieger_oxide::{
    bitmap::{Bitmap, Bump, Cell, GlowRect, Operation, Pixel, Transform, Twirl, fade64, inner},
    random::Noise,
};

#[test]
fn every_inner_mode_matches_original_mmx_bit_for_bit() {
    let bytes = include_bytes!("fixtures/bitmap-inner.bin");
    let mut pos = 0;
    fn word(bytes: &[u8], pos: &mut usize) -> u32 {
        let w = u32::from_le_bytes(bytes[*pos..*pos + 4].try_into().unwrap());
        *pos += 4;
        w
    }
    fn pixels(bytes: &[u8], pos: &mut usize, count: usize) -> Vec<Pixel> {
        (0..count)
            .map(|_| {
                std::array::from_fn(|_| {
                    let w = u16::from_le_bytes(bytes[*pos..*pos + 2].try_into().unwrap());
                    *pos += 2;
                    w
                })
            })
            .collect()
    }
    let cases = word(bytes, &mut pos);
    assert_eq!(cases, 40);
    for case in 0..cases {
        let mode = word(bytes, &mut pos) as u8;
        let count = word(bytes, &mut pos) as usize;
        let mut d = pixels(bytes, &mut pos, count);
        let s = pixels(bytes, &mut pos, count);
        let x = pixels(bytes, &mut pos, count);
        let expected = pixels(bytes, &mut pos, count);
        inner(&mut d, &s, &x, Operation::try_from(mode).unwrap()).unwrap();
        for (i, (a, b)) in d.iter().zip(&expected).enumerate() {
            assert_eq!(a, b, "mode {mode}, case {case}, pixel {i}");
        }
    }
    assert_eq!(pos, bytes.len());
}

#[test]
fn packed_interpolation_matches_original_mmx_bit_for_bit() {
    let b = include_bytes!("fixtures/bitmap-fade.bin");
    let count = u32::from_le_bytes(b[..4].try_into().unwrap()) as usize;
    assert_eq!(count, 512);
    assert_eq!(b.len(), 4 + count * 28);
    for (i, row) in b[4..].as_chunks::<28>().0.iter().enumerate() {
        let pixel = |offset| {
            std::array::from_fn(|c| {
                u16::from_le_bytes(row[offset + c * 2..offset + c * 2 + 2].try_into().unwrap())
            })
        };
        let (a, c) = (pixel(0), pixel(8));
        let fade = i32::from_le_bytes(row[16..20].try_into().unwrap());
        assert_eq!(fade64(a, c, fade), pixel(20), "fade case {i}");
    }
}

#[test]
fn scalar_generators_match_all_original_parameter_sets() {
    verify_generators(include_bytes!("fixtures/generators3383.bin"), 966);
}

#[test]
#[ignore = "requires original beta coefficient fixtures generated locally"]
fn original_beta_generators_match_all_original_parameter_sets() {
    verify_generators(
        &std::fs::read(".reference/generators-beta.bin").unwrap(),
        780,
    );
}

fn verify_generators(bytes: &[u8], expected_count: u32) {
    let mut pos = 0;
    fn word(b: &[u8], p: &mut usize) -> u32 {
        let w = u32::from_le_bytes(b[*p..*p + 4].try_into().unwrap());
        *p += 4;
        w
    }
    fn pixels(b: &[u8], p: &mut usize, n: usize) -> Vec<Pixel> {
        (0..n)
            .map(|_| {
                std::array::from_fn(|_| {
                    let w = u16::from_le_bytes(b[*p..*p + 2].try_into().unwrap());
                    *p += 2;
                    w
                })
            })
            .collect()
    }
    let count = word(bytes, &mut pos);
    assert_eq!(count, expected_count);
    let noise = Noise::new();
    for _ in 0..count {
        let id = word(bytes, &mut pos);
        let class = word(bytes, &mut pos);
        let count = word(bytes, &mut pos);
        let p: Vec<_> = (0..count).map(|_| word(bytes, &mut pos)).collect();
        let int = |i: usize| p[i] as i32;
        let f = |i: usize| f32::from_bits(p[i]);
        let w = word(bytes, &mut pos) as usize;
        let h = word(bytes, &mut pos) as usize;
        let input_count = word(bytes, &mut pos) as usize;
        let input = pixels(bytes, &mut pos, input_count);
        let expected = pixels(bytes, &mut pos, w * h);
        let image = |data: Vec<Pixel>| {
            let mut b = Bitmap::flat(w.ilog2() as i32, h.ilog2() as i32, 0, 0);
            b.pixels = data;
            b
        };
        let actual = match class {
            0x22 => Bitmap::perlin(
                int(0),
                int(1),
                int(2),
                int(3),
                f(4),
                int(5) as u8,
                p[6],
                f(7),
                f(8),
                p[9],
                p[10],
                0,
                &noise,
            )
            .unwrap(),
            0x2b => {
                let mut b = image(input);
                b.hscb(f(0), f(1), f(2), f(3));
                b
            }
            0x27 | 0x39 => {
                let mut b = image(input);
                b.glow(GlowRect {
                    center: [f(0), f(1)],
                    radius: [f(2), f(3)],
                    solid: [f(4), f(5)],
                    color: p[6],
                    alpha: f(7),
                    power: f(8),
                    wrap: p[9],
                    legacy: p[10] != 0,
                });
                b
            }
            0x29 => {
                let mut b = image(input);
                b.blur(p[0], f(1), f(2), f(3)).unwrap();
                b
            }
            0x2c => {
                let mut b = image(input);
                b.rotate(
                    Transform {
                        angle: f(0),
                        scale: [f(1), f(2)],
                        translate: [f(3), f(4)],
                        border: p[5],
                        new_size: [
                            p.get(6).copied().unwrap_or(0) as i32,
                            p.get(7).copied().unwrap_or(0) as i32,
                        ],
                    },
                    0,
                )
                .unwrap();
                b
            }
            0x2d => {
                assert_eq!(input.len(), w * h * 2);
                let a = image(input[..w * h].to_vec());
                let mut field = image(input[w * h..].to_vec());
                field.distort(&a, f(0), p[1]).unwrap();
                field
            }
            0x2e => {
                let mut b = image(input);
                b.normals(f(0), p[1]);
                b
            }
            0x30 => {
                assert_eq!(input.len(), w * h * 2);
                let mut b = image(input[..w * h].to_vec());
                let n = image(input[w * h..].to_vec());
                b.bump(
                    Some(&n),
                    Bump {
                        kind: p[0],
                        position: [f(1), f(2), f(3)],
                        direction: [f(4), f(5)],
                        diffuse: p[6],
                        ambient: p[7],
                        outer: f(8),
                        falloff: f(9),
                        amplitude: f(10),
                        specular: p[11],
                        specular_power: f(12),
                        specular_amplitude: f(13),
                    },
                )
                .unwrap();
                b
            }
            0x32 => Bitmap::cell(
                int(0),
                int(1),
                Cell {
                    colors: [p[2], p[3], p[4]],
                    count: p[5] as usize,
                    seed: int(6),
                    amplitude: f(7),
                    gamma: f(8),
                    mode: p[9],
                    min_distance: f(10),
                    percent: int(11),
                    aspect: f(12),
                },
                0,
            )
            .unwrap(),
            0x34 => {
                Bitmap::gradient(int(0), int(1), p[2], p[3], f(4), f(5), f(6), p[7], 0).unwrap()
            }
            0x36 => {
                let mut b = image(input);
                b.rotate_mul(
                    Transform {
                        angle: f(0),
                        scale: [f(1), f(2)],
                        translate: [f(3), f(4)],
                        border: p[5],
                        new_size: [0; 2],
                    },
                    p[6],
                    p[7],
                    p[8],
                    p[9],
                    0,
                )
                .unwrap();
                b
            }
            0x37 => {
                let mut b = image(input);
                b.twirl(Twirl {
                    strength: f(0),
                    gamma: f(1),
                    radius: [f(2), f(3)],
                    center: [f(4), f(5)],
                    border: p[6],
                })
                .unwrap();
                b
            }
            0x38 => {
                let mut b = image(input);
                b.sharpen(p[0], f(1), f(2), f(3)).unwrap();
                b
            }
            _ => panic!("unknown fixture class"),
        };
        assert_eq!((actual.width, actual.height), (w, h));
        for (pixel, (a, b)) in actual.pixels.iter().zip(expected).enumerate() {
            assert_eq!(*a, b, "op {id}, class {class:x}, pixel {pixel}");
        }
    }
    assert_eq!(pos, bytes.len());
}

#[test]
fn bilinear_sampling_matches_original_mmx_at_borders_and_extreme_coordinates() {
    let bytes = include_bytes!("fixtures/bitmap-bilinear.bin");
    let word = |p: usize| u32::from_le_bytes(bytes[p..p + 4].try_into().unwrap());
    let (count, w, h) = (word(0) as usize, word(4) as usize, word(8) as usize);
    let mut bitmap = Bitmap::flat(3, 2, 0, 0);
    assert_eq!((w, h), (bitmap.width, bitmap.height));
    let mut pos = 12;
    for p in &mut bitmap.pixels {
        for v in p {
            *v = u16::from_le_bytes(bytes[pos..pos + 2].try_into().unwrap());
            pos += 2;
        }
    }
    for i in 0..count {
        let (u, v, border) = (word(pos) as i32, word(pos + 4) as i32, word(pos + 8));
        pos += 12;
        let expected = std::array::from_fn(|_| {
            let v = u16::from_le_bytes(bytes[pos..pos + 2].try_into().unwrap());
            pos += 2;
            v
        });
        assert_eq!(bitmap.sample(u, v, border), expected, "sample {i}");
    }
    assert_eq!(pos, bytes.len());
}
