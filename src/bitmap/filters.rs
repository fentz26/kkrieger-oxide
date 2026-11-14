//! Farbrausch's original spatial texture operators, translated to safe Rust.
use super::*;

#[derive(Clone, Copy)]
pub struct GlowRect {
    pub center: [f32; 2],
    pub radius: [f32; 2],
    pub solid: [f32; 2],
    pub color: u32,
    pub alpha: f32,
    pub power: f32,
    pub wrap: u32,
    pub legacy: bool,
}
#[derive(Clone, Copy)]
pub struct Transform {
    pub angle: f32,
    pub scale: [f32; 2],
    pub translate: [f32; 2],
    pub border: u32,
    pub new_size: [i32; 2],
}
#[derive(Clone, Copy)]
pub struct Twirl {
    pub strength: f32,
    pub gamma: f32,
    pub radius: [f32; 2],
    pub center: [f32; 2],
    pub border: u32,
}
#[derive(Clone, Copy)]
pub struct Cell {
    pub colors: [u32; 3],
    pub count: usize,
    pub seed: i32,
    pub amplitude: f32,
    pub gamma: f32,
    pub mode: u32,
    pub min_distance: f32,
    pub percent: i32,
    pub aspect: f32,
}

impl Bitmap {
    pub fn glow(&mut self, g: GlowRect) {
        let [cx, cy] = g.center;
        let [rx, ry] = g.radius;
        let [sx, sy] = g.solid;
        if g.wrap == 1 {
            if cx + rx + sx > 1. {
                self.glow(GlowRect {
                    center: [cx - 1., cy],
                    wrap: 2,
                    ..g
                });
            }
            if cx - rx - sx < 0. {
                self.glow(GlowRect {
                    center: [cx + 1., cy],
                    wrap: 2,
                    ..g
                });
            }
        }
        if g.wrap == 1 || g.wrap == 2 {
            if cy + ry + sy > 1. {
                self.glow(GlowRect {
                    center: [cx, cy - 1.],
                    wrap: 0,
                    ..g
                });
            }
            if cy - ry - sy < 0. {
                self.glow(GlowRect {
                    center: [cx, cy + 1.],
                    wrap: 0,
                    ..g
                });
            }
        }
        let power = if g.power == 0. { 1. / 65536. } else { g.power };
        let power = (0.25_f64 / power as f64) as f32;
        let cx = cx * self.width as f32;
        let cy = cy * self.height as f32;
        let sx = sx * self.width as f32;
        let sy = sy * self.height as f32;
        let rx = (rx * self.width as f32).max(1. / 65536.);
        let ry = (ry * self.height as f32).max(1. / 65536.);
        let rx = 1. / (rx * rx);
        let ry = 1. / (ry * ry);
        let alpha = g.alpha * 32768.;
        let col = color64(g.color);
        let curve = |x: f32| {
            let v = if g.legacy {
                (1. - x).powf64(power)
            } else {
                1. - x.powf64(power * 2.)
            };
            ((v * alpha as f64) as i32).clamp(0, 32767) * 2
        };
        let table: [i32; 1025] = std::array::from_fn(|i| curve(i as f32 / 1024.));
        let low: [i32; 32] = std::array::from_fn(|i| curve(i as f32 / 32768.));
        for y in 0..self.height {
            let mut fy = (y as f32 - cy).abs() - sy;
            fy = fy.max(0.);
            let weighted = fy * ry;
            fy *= weighted;
            for x in 0..self.width {
                let fx = ((x as f32 - cx).abs() - sx).max(0.);
                let a = fx * fx * rx + fy;
                if a < 1. - 1. / 32768. {
                    let index = (a * 32768.).round_ties_even() as i32;
                    let f = if index < 32 {
                        low[index as usize]
                    } else {
                        gamma(&table, index)
                    };
                    let pixel = &mut self.pixels[y * self.width + x];
                    *pixel = fade64(*pixel, col, f);
                }
            }
        }
    }

    /// Original fixed-point bilinear sampling, including its low-end bias clamp.
    pub fn sample(&self, u: i32, v: i32, border: u32) -> Pixel {
        let coordinate = |i: i32, size: usize, clamp: bool| {
            if clamp {
                i.clamp(0, size as i32 - 1) as usize
            } else {
                (i as usize) & (size - 1)
            }
        };
        let x = u >> 16;
        let y = v >> 16;
        let x0 = coordinate(x, self.width, border & 1 != 0);
        let x1 = coordinate(x + 1, self.width, border & 1 != 0);
        let y0 = coordinate(y, self.height, border & 2 != 0);
        let y1 = coordinate(y + 1, self.height, border & 2 != 0);
        let a = self.pixels[y0 * self.width + x0];
        let b = self.pixels[y0 * self.width + x1];
        let c = self.pixels[y1 * self.width + x0];
        let d = self.pixels[y1 * self.width + x1];
        let fx = ((u as u32 & 65535) >> 1) as u16;
        let fy = ((v as u32 & 65535) >> 1) as u16;
        std::array::from_fn(|i| {
            let top = a[i].wrapping_add(multiply(b[i].wrapping_sub(a[i]), fx));
            let bottom = c[i].wrapping_add(multiply(d[i].wrapping_sub(c[i]), fx));
            top.wrapping_add(0x8000)
                .wrapping_add(multiply(bottom.wrapping_sub(top), fy))
                .saturating_sub(0x8000)
        })
    }

    pub fn rotate(&mut self, t: Transform, size_offset: i32) -> Result<()> {
        let [sx, sy] = t.scale;
        if sx == 0. || sy == 0. {
            return Err(Error("zero texture scale"));
        }
        let target = |specified: i32, current: usize| -> Result<usize> {
            if specified == 0 {
                return Ok(current);
            }
            let power = specified - 1 + size_offset;
            if !(0..=12).contains(&power) {
                return Err(Error("invalid rotated texture dimensions"));
            }
            Ok(1 << power)
        };
        let txs = target(t.new_size[0], self.width)?;
        let tys = target(t.new_size[1], self.height)?;
        let angle = t.angle * std::f32::consts::TAU;
        let sin = (angle as f64).sin();
        let cos = (angle as f64).cos();
        let round = |f: f64| (f as f32).round_ties_even() as i32;
        let m00 = round(cos * 65536. * self.width as f64 / (sx * txs as f32) as f64);
        let m01 = round(sin * 65536. * self.height as f64 / (sx * txs as f32) as f64);
        let m10 = round(-sin * 65536. * self.width as f64 / (sy * tys as f32) as f64);
        let m11 = round(cos * 65536. * self.height as f64 / (sy * tys as f32) as f64);
        let halfx = ((txs as i32 - 1)
            .wrapping_mul(m00)
            .wrapping_add((tys as i32 - 1).wrapping_mul(m10)))
            / 2;
        let halfy = ((txs as i32 - 1)
            .wrapping_mul(m01)
            .wrapping_add((tys as i32 - 1).wrapping_mul(m11)))
            / 2;
        let m20 =
            (t.translate[0] * self.width as f32 * 65536. - halfx as f32).round_ties_even() as i32;
        let m21 =
            (t.translate[1] * self.height as f32 * 65536. - halfy as f32).round_ties_even() as i32;
        let mut out = Vec::with_capacity(txs * tys);
        for y in 0..tys {
            let mut u = (y as i32).wrapping_mul(m10).wrapping_add(m20);
            let mut v = (y as i32).wrapping_mul(m11).wrapping_add(m21);
            for _ in 0..txs {
                out.push(self.sample(u, v, t.border));
                u = u.wrapping_add(m00);
                v = v.wrapping_add(m01);
            }
        }
        self.pixels = out;
        self.width = txs;
        self.height = tys;
        Ok(())
    }

    pub fn twirl(&mut self, t: Twirl) -> Result<()> {
        if t.radius.contains(&0.) {
            return Err(Error("zero twirl radius"));
        }
        let rx = t.radius[0] * 0.5;
        let ry = t.radius[1] * 0.5;
        let strength = t.strength * std::f32::consts::TAU;
        let sin: [i32; 1025] = std::array::from_fn(|i| {
            let angle = ((i as f32 / 1024.).powf64(t.gamma) * strength as f64) as f32;
            (65536. * (angle as f64).sin() as f32) as i32
        });
        let cos: [i32; 1025] = std::array::from_fn(|i| {
            let angle = ((i as f32 / 1024.).powf64(t.gamma) * strength as f64) as f32;
            (65536. * (angle as f64).cos() as f32) as i32
        });
        let lookup = |table: &[i32; 1025], d: i32| {
            let i = (d >> 6) as usize;
            table[i] + (((table[i + 1] - table[i]) * (d & 63)) >> 6)
        };
        let cx = (t.center[0] * 65536.) as i32;
        let cy = (t.center[1] * 65536.) as i32;
        let rx = (rx * 65536.) as i32;
        let ry = (ry * 65536.) as i32;
        if rx == 0 || ry == 0 {
            return Err(Error("twirl radius below fixed-point precision"));
        }
        let xstep = 65536 / self.width as i32;
        let ystep = 65536 / self.height as i32;
        let mut out = Vec::with_capacity(self.pixels.len());
        for y in 0..self.height {
            let py = y as i32 * ystep;
            let dy = py - cy;
            let base = 65536 - ((dy as i64 * dy as i64) / ry as i64) as i32;
            for x in 0..self.width {
                let px = x as i32 * xstep;
                let dx = px - cx;
                let distance = base - ((dx as i64 * dx as i64) / rx as i64) as i32;
                let (u, v) = if distance > 0 {
                    if distance >= 65536 {
                        // The exact center samples the last table entry.
                        (
                            cx + mul_shift(dx, cos[1024]) + mul_shift(dy, sin[1024]),
                            cy - mul_shift(dx, sin[1024]) + mul_shift(dy, cos[1024]),
                        )
                    } else {
                        let (s, c) = (lookup(&sin, distance), lookup(&cos, distance));
                        (
                            cx + mul_shift(dx, c) + mul_shift(dy, s),
                            cy - mul_shift(dx, s) + mul_shift(dy, c),
                        )
                    }
                } else {
                    (px, py)
                };
                out.push(self.sample(
                    u.wrapping_mul(self.width as i32),
                    v.wrapping_mul(self.height as i32),
                    t.border,
                ));
            }
        }
        self.pixels = out;
        Ok(())
    }

    /// `image` is the first original input; `self` is its displacement field.
    pub fn distort(&mut self, image: &Self, distance: f32, border: u32) -> Result<()> {
        self.check_size(image)?;
        let bx = (distance * self.width as f32 * 4.) as i32;
        let by = (distance * self.height as f32 * 4.) as i32;
        let mut out = Vec::with_capacity(self.pixels.len());
        // Preserve the original traversal for rectangular textures as well.
        for y in 0..self.width {
            for x in 0..self.height {
                let p = self.pixels[out.len()];
                let u = (x as i32)
                    .wrapping_shl(16)
                    .wrapping_add((p[0] as i32 - 16384).wrapping_mul(bx));
                let v = (y as i32)
                    .wrapping_shl(16)
                    .wrapping_add((p[1] as i32 - 16384).wrapping_mul(by));
                out.push(image.sample(u, v, border));
            }
        }
        self.pixels = out;
        Ok(())
    }

    pub fn normals(&mut self, distance: f32, mode: u32) {
        let dist = (distance * 65536.).round_ties_even() as i32;
        let sx = self.width.ilog2();
        let sy = self.height.ilog2();
        let mut out = Vec::with_capacity(self.pixels.len());
        for y in 0..self.height {
            for x in 0..self.width {
                let horizontal = |d: isize| {
                    self.pixels[y * self.width + (x.wrapping_add_signed(d) & (self.width - 1))][0]
                        as i32
                };
                let vertical = |d: isize| {
                    self.pixels[(y.wrapping_add_signed(d) & (self.height - 1)) * self.width + x][0]
                        as i32
                };
                let vx = horizontal(-2) + horizontal(-1) * 3 - horizontal(0) * 3 - horizontal(1);
                let vy = vertical(-2) + vertical(-1) * 3 - vertical(0) * 3 - vertical(1);
                let mut vx =
                    ((vx.wrapping_mul(dist >> 4) >> (20 - sx)) + 16384).clamp(0, 32767) - 16384;
                let mut vy =
                    ((vy.wrapping_mul(dist >> 4) >> (20 - sy)) + 16384).clamp(0, 32767) - 16384;
                let mut vz = 0;
                if mode & 1 != 0 {
                    vz = 16383 * 16383 - vx * vx - vy * vy;
                    if vz > 0 {
                        vz = (vz as f64).sqrt() as i32;
                    } else {
                        let e = (1. / ((vx * vx + vy * vy) as f64).sqrt() * 16383.) as f32;
                        vx = (vx as f32 * e) as i32;
                        vy = (vy as f32 * e) as i32;
                        vz = 0;
                    }
                }
                if mode & 2 != 0 {
                    std::mem::swap(&mut vx, &mut vy);
                    vy = -vy;
                }
                out.push([
                    (vx + 16384) as u16,
                    (vy + 16384) as u16,
                    (vz + 16384) as u16,
                    65535,
                ]);
            }
        }
        self.pixels = out;
    }

    pub fn blur(&mut self, flags: u32, sx: f32, sy: f32, amplitude: f32) -> Result<()> {
        let order = flags & 15;
        if order == 0 {
            return Ok(());
        }
        if sx < 0. || sy < 0. || !amplitude.is_finite() {
            return Err(Error("invalid blur parameters"));
        }
        let sizes = [
            (128. * sx * self.width as f32).round_ties_even() as i32,
            (128. * sy * self.height as f32).round_ties_even() as i32,
        ];
        let famp = (amplitude * 65536. * 64.) as i32;
        let mut scratch = vec![[0; 4]; self.pixels.len()];
        for fixed_size in sizes {
            let f1 = (fixed_size & 127) / 2;
            let f0 = 64 - f1;
            let size = (fixed_size / 128) * 2 + if flags & 16 != 0 { 1 } else { 0 };
            let amp = famp / (size * 64 + f1 * 2).max(1);
            let clip = if amp > 128 {
                div_shift(65536 * 64, amp) - 1
            } else {
                i32::MAX
            };
            let lo = amp as u16;
            let hi = (amp >> 16) as u16;
            for _ in 0..order {
                for y in 0..self.height {
                    let row = &self.pixels[y * self.width..(y + 1) * self.width];
                    let at = |x: i32| row[(x as usize) & (self.width - 1)];
                    let mut left = -((size + 1) / 2);
                    let mut right = left;
                    let mut acc = at(left).map(|v| (v as i16 as i32).wrapping_mul(f1 - f0));
                    let pair = |x: i32, reverse: bool| {
                        let a = at(x);
                        let b = at(x + 1);
                        std::array::from_fn::<_, 4, _>(|c| {
                            if reverse {
                                (b[c] as i16 as i32) * f0 + (a[c] as i16 as i32) * f1
                            } else {
                                (a[c] as i16 as i32) * f0 + (b[c] as i16 as i32) * f1
                            }
                        })
                    };
                    for _ in 0..size {
                        let p = pair(right, false);
                        for c in 0..4 {
                            acc[c] = acc[c].wrapping_add(p[c]);
                        }
                        right += 1;
                    }
                    for x in 0..self.width {
                        let p = pair(right, false);
                        for c in 0..4 {
                            acc[c] = acc[c].wrapping_add(p[c]);
                        }
                        right += 1;
                        scratch[y * self.width + x] = acc.map(|a| {
                            let akhi = signed_sat(((a as u32) >> 22) as i32);
                            let aklo = signed_sat(a.wrapping_shl(10) >> 16);
                            let mut out = if a > clip { 65535_u16 } else { 0 };
                            out = out.saturating_add(akhi.wrapping_mul(lo));
                            out = out.saturating_add(aklo.wrapping_mul(hi));
                            out = out.saturating_add(((aklo as u32 * lo as u32) >> 16) as u16);
                            out.min(32767)
                        });
                        let p = pair(left, true);
                        for c in 0..4 {
                            acc[c] = acc[c].wrapping_sub(p[c]);
                        }
                        left += 1;
                    }
                }
                std::mem::swap(&mut self.pixels, &mut scratch);
            }
            for y in 0..self.height {
                for x in 0..self.width {
                    scratch[x * self.height + y] = self.pixels[y * self.width + x];
                }
            }
            std::mem::swap(&mut self.pixels, &mut scratch);
            std::mem::swap(&mut self.width, &mut self.height);
        }
        Ok(())
    }

    pub fn sharpen(&mut self, order: u32, sx: f32, sy: f32, amplitude: f32) -> Result<()> {
        let original = self.pixels.clone();
        self.blur(order | 16, sx, sy, 1.)?;
        let word = ((amplitude * 2048.) as i32).clamp(-32767, 32767) as i16 as u16;
        for (p, &x) in self.pixels.iter_mut().zip(&original) {
            *p = process(Operation::Sharpen, *p, [word; 4], x, [0; 4]);
        }
        Ok(())
    }

    pub fn cell(xs: i32, ys: i32, p: Cell, offset: i32) -> Result<Self> {
        if !(1..=256).contains(&p.count) {
            return Err(Error("invalid cell count"));
        }
        let mut bm = Self::flat(xs, ys, 0, offset);
        let mut rng = Random::new(p.seed);
        let mut cells: Vec<[i32; 3]> = (0..p.count)
            .map(|_| std::array::from_fn(|_| rng.below(16384) as i32))
            .collect();
        let min_distance = (p.min_distance * 16384.).round_ties_even() as i32;
        let min_distance = min_distance.wrapping_mul(min_distance);
        let mut count = p.count;
        let mut i = 1;
        while i < count {
            if p.mode & 2 != 0 && (rng.below(255) as i32) < p.percent {
                cells[i][2] = 65535;
            }
            let x = (cells[i][0] & 16383) - 8192;
            let y = (cells[i][1] & 16383) - 8192;
            let cut = (0..i).any(|j| {
                let dx = ((cells[j][0] - x) & 16383) - 8192;
                let dy = ((cells[j][1] - y) & 16383) - 8192;
                dx * dx + dy * dy < min_distance
            });
            if cut {
                count -= 1;
                cells[i] = cells[count];
            } else {
                i += 1;
            }
        }
        let shiftx = 14 - bm.width.ilog2();
        let shifty = 14 - bm.height.ilog2();
        let a = color64(p.colors[1]);
        let b = color64(p.colors[0]);
        let background = color64(p.colors[2]);
        let aspect = (2_f64.powf(p.aspect as f64)) as f32;
        let square = aspect * aspect;
        let divisor = 1. / (16384. * aspect);
        let mut output = 0;
        for y in 0..bm.width {
            let py = (y as i32) << shifty;
            let vertical: Vec<_> = cells[..count]
                .iter()
                .map(|c| {
                    let dy = ((c[1] - py) & 16383) - 8192;
                    ((dy * dy) as f32 * square) as i32
                })
                .collect();
            for x in 0..bm.height {
                let px = (x as i32) << shiftx;
                let mut best = 32768 * 32768;
                let mut second = best;
                let mut closest = 0;
                for j in 0..count {
                    let dx = ((cells[j][0] - px) & 16383) - 8192;
                    let distance = (dx * dx).wrapping_add(vertical[j]);
                    if distance < best {
                        second = best;
                        best = distance;
                        closest = j;
                    } else if distance < second {
                        second = distance;
                    }
                }
                let mut v = ((best as f64).sqrt() * divisor as f64) as f32;
                if p.mode & 1 != 0 {
                    let v1 = ((second as f64).sqrt() * divisor as f64) as f32;
                    v = if v + v1 > 0.00001 {
                        (v1 - v) / (v1 + v)
                    } else {
                        0.
                    };
                }
                let mut f =
                    (((v * p.amplitude).powf64(p.gamma) * 32768.) as i32).clamp(0, 32767) * 2;
                if p.mode & 4 != 0 {
                    f = 65536 - f;
                }
                bm.pixels[output] = if p.mode & 2 != 0 {
                    let color = if cells[closest][2] == 65535 {
                        background
                    } else {
                        fade64(a, b, cells[closest][2] * 4)
                    };
                    fade64(color, background, f)
                } else {
                    fade64(a, b, f)
                };
                output += 1;
            }
        }
        Ok(bm)
    }
}

#[derive(Clone, Copy)]
pub struct Bump {
    pub kind: u32,
    pub position: [f32; 3],
    pub direction: [f32; 2],
    pub diffuse: u32,
    pub ambient: u32,
    pub specular: u32,
    pub outer: f32,
    pub falloff: f32,
    pub amplitude: f32,
    pub specular_power: f32,
    pub specular_amplitude: f32,
}

impl Bitmap {
    pub fn rotate_mul(
        &mut self,
        mut t: Transform,
        color: u32,
        mode: u32,
        mut count: u32,
        mut fade: u32,
        offset: i32,
    ) -> Result<()> {
        if count == 0 || count > 30 {
            return Err(Error("invalid repeated rotation count"));
        }
        self.color(0, color)?;
        let mut background = self.clone();
        let sign = t.scale.map(|v| if v < 0. { -1. } else { 1. });
        t.scale[0] *= sign[0];
        t.scale[1] *= sign[1];
        let divisor = if mode & 16 != 0 {
            (1_u32 << count) - 1
        } else {
            count
        };
        t.angle /= if mode & 16 != 0 {
            (1_u32 << count) as f32
        } else {
            count as f32
        };
        for (i, &sign) in sign.iter().enumerate() {
            t.translate[i] = ((t.translate[i] as f64 - 0.5) / divisor as f64 + 0.5) as f32;
            let exponent = 1. / divisor as f32;
            t.scale[i] = (t.scale[i].powf64(exponent)) as f32 * sign;
        }
        let mut i = 1;
        while count > 0 {
            count -= 1;
            if mode & 16 != 0 {
                if fade != 0xffffffff {
                    self.color(0, fade)?;
                    fade = (0..4).fold(0, |c, i| {
                        let v = (fade >> (i * 8)) & 255;
                        c | (((v * v) >> 8) << (i * 8))
                    });
                }
                self.rotate(t, offset)?;
                self.merge(&background, mode & 15)?;
                background = self.clone();
                t.angle *= 2.;
                for c in 0..2 {
                    t.translate[c] = (t.translate[c] - 0.5) * 2. + 0.5;
                    t.scale[c] *= t.scale[c];
                }
            } else {
                if fade != 0xffffffff {
                    self.color(0, fade)?;
                }
                let mut transformed = self.clone();
                transformed.rotate(
                    Transform {
                        angle: t.angle * i as f32,
                        scale: t.scale.map(|v| (v - 1.) * i as f32 + 1.),
                        translate: t
                            .translate
                            .map(|v| ((v as f64 - 0.5) * i as f64 + 0.5) as f32),
                        ..t
                    },
                    offset,
                )?;
                background.merge(&transformed, mode & 15)?;
                i += 1;
            }
        }
        *self = background;
        Ok(())
    }

    pub fn bump(&mut self, normals: Option<&Self>, p: Bump) -> Result<()> {
        if let Some(n) = normals {
            self.check_size(n)?;
        }
        if p.outer == 0. {
            return Err(Error("zero bump spotlight outer angle"));
        }
        let diffuse = color64(p.diffuse);
        let ambient = color64(p.ambient);
        let specular = color64(p.specular);
        let [mut px, mut py, mut pz] = p.position;
        px = px * self.width as f32 * 2. - (self.width / 2) as f32;
        py = py * self.height as f32 * 2. - (self.height / 2) as f32;
        pz *= self.width as f32;
        let da = p.direction[0] * std::f32::consts::TAU;
        let db = p.direction[1] * std::f32::consts::PI;
        let base = (db as f64).cos() as f32;
        let dx = (base as f64 * (da as f64).sin()) as f32;
        let dy = (base as f64 * (da as f64).cos()) as f32;
        let dz = (db as f64).sin() as f32;
        if p.kind == 0 {
            px -= dx * pz / dz;
            py -= dy * pz / dz;
        }
        let amplitude = p.specular_amplitude * 65536.;
        for y in 0..self.height {
            for x in 0..self.width {
                let (lx, ly, lz) = if p.kind != 2 {
                    let (lx, ly, lz) = (x as f32 - px, y as f32 - py, pz);
                    let e = (1. / ((lx * lx + ly * ly + lz * lz) as f64).sqrt()) as f32;
                    (lx * e, ly * e, lz * e)
                } else {
                    (dx, dy, dz)
                };
                let n = normals.map(|n| n.pixels[y * self.width + x]);
                let (nx, ny, nz) = n
                    .map(|n| {
                        (
                            (n[0] as i32 - 16384) as f32 / 16384.,
                            (n[1] as i32 - 16384) as f32 / 16384.,
                            (n[2] as i32 - 16384) as f32 / 16384.,
                        )
                    })
                    .unwrap_or((0., 0., 1.));
                let lf = lx * nx + ly * ny + lz * nz;
                let sf = if amplitude != 0. {
                    let hz = lz + 1.;
                    let e = (1. / ((lx * lx + ly * ly + hz * hz) as f64).sqrt()) as f32;
                    let s = (lx * nx + ly * ny + hz * nz).max(0.);
                    (s * e).powf64(p.specular_power) as f32
                } else {
                    0.
                };
                let df = if p.kind == 0 {
                    let d = lx * dx + ly * dy + lz * dz;
                    if d < p.outer {
                        0.
                    } else {
                        ((d - p.outer) / (1. - p.outer)).powf64(p.falloff) as f32
                    }
                } else {
                    1.
                };
                let f = df * lf * p.amplitude;
                let pixel = &mut self.pixels[y * self.width + x];
                let fade = (df * sf * amplitude).round_ties_even() as i32;
                let weight = signed_sat(fade >> 1);
                for c in 0..4 {
                    let value = (pixel[c] as f32 * (ambient[c] as f32 + diffuse[c] as f32 * f)
                        / 32768.)
                        .round_ties_even() as i32;
                    pixel[c] =
                        add_signed(value.clamp(0, 32767) as u16, multiply(specular[c], weight));
                }
            }
        }
        Ok(())
    }
}

trait Pow64 {
    fn powf64(self, power: f32) -> f64;
}
impl Pow64 for f32 {
    fn powf64(self, power: f32) -> f64 {
        (self as f64).powf(power as f64)
    }
}
