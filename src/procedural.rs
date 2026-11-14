use crate::math::V3;

pub const TILE: usize = 128;
pub const TILES: usize = 4;

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Vertex {
    pub position: V3,
    pub normal: V3,
    pub uv: [f32; 2],
    pub material: f32,
}

#[derive(Clone, Copy)]
pub struct Solid {
    pub min: V3,
    pub max: V3,
}

impl Solid {
    pub fn blocks(&self, x: f32, z: f32, radius: f32) -> bool {
        let dx = x - x.clamp(self.min[0], self.max[0]);
        let dz = z - z.clamp(self.min[2], self.max[2]);
        dx * dx + dz * dz < radius * radius && self.max[1] > 0.25 && self.min[1] < 1.65
    }
}

pub struct Room {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    pub solids: Vec<Solid>,
}

impl Room {
    pub fn generate() -> Self {
        let mut room = Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            solids: Vec::new(),
        };
        // The level is a recipe: dimensions, repetition, materials. No imported assets.
        room.cube([-6., -0.3, -20.], [6., 0., 6.], 1., false);
        room.cube([-6., 4.8, -20.], [6., 5.1, 6.], 0., false);
        room.cube([-6.3, 0., -20.], [-6., 4.8, 6.], 0., true);
        room.cube([6., 0., -20.], [6.3, 4.8, 6.], 0., true);
        room.cube([-6., 0., -20.3], [6., 4.8, -20.], 0., true);
        room.cube([-6., 0., 6.], [6., 4.8, 6.3], 0., true);
        for z in [-18., -12., -6., 0., 5.] {
            for x in [-5.6, 5.6] {
                room.cube([x - 0.3, 0., z - 0.4], [x + 0.3, 4.8, z + 0.4], 2., true);
                room.cube(
                    [x - 0.36, 0.1, z - 0.46],
                    [x + 0.36, 0.4, z + 0.46],
                    2.,
                    true,
                );
                room.cube(
                    [x - 0.32, 2.1, z - 0.43],
                    [x + 0.32, 2.18, z + 0.43],
                    3.,
                    false,
                );
            }
            room.cube([-6., 4.25, z - 0.3], [6., 4.8, z + 0.3], 2., false);
            room.cube([-3.8, 4.23, z - 0.09], [3.8, 4.25, z + 0.09], 3., false);
        }
        for x in [-2.8, 2.8] {
            room.cube(
                [x - 0.035, 0.002, -19.8],
                [x + 0.035, 0.012, 5.8],
                3.,
                false,
            );
        }
        // Central reactor, plinth and luminous core.
        room.cube([-1.6, 0., -13.6], [1.6, 0.35, -10.4], 2., true);
        room.cube([-0.9, 0.35, -12.9], [0.9, 3.7, -11.1], 3., true);
        for x in [-1.1, 0.9] {
            for z in [-13.1, -11.1] {
                room.cube([x, 0.35, z], [x + 0.2, 4.1, z + 0.2], 2., true);
            }
        }
        room.cube([-1.5, 3.7, -13.5], [1.5, 4.1, -10.5], 2., false);
        // Equipment stacks deliberately leave a navigable route on both sides.
        for (x, z) in [(-4.3, -7.8), (3.9, -3.8), (-4.5, -17.)] {
            room.cube([x - 0.8, 0., z - 0.7], [x + 0.8, 1.25, z + 0.7], 2., true);
            room.cube([x - 0.7, 1.25, z - 0.6], [x + 0.7, 1.35, z + 0.6], 0., true);
            room.cube(
                [x - 0.4, 0.8, z + 0.702],
                [x + 0.4, 0.88, z + 0.708],
                3.,
                false,
            );
        }
        // Door at the far end and recessed wall panels.
        room.cube([-1.7, 0., -19.97], [1.7, 3.8, -19.8], 2., true);
        room.cube([-0.035, 0.1, -19.79], [0.035, 3.65, -19.78], 3., false);
        for z in [-15., -9., -3., 3.] {
            for x in [-5.99, 5.94] {
                room.cube([x, 1.25, z - 1.1], [x + 0.05, 2.85, z + 1.1], 2., false);
                room.cube(
                    [x - 0.005, 2.5, z - 0.9],
                    [x + 0.055, 2.55, z + 0.9],
                    3.,
                    false,
                );
            }
        }
        room
    }

    fn cube(&mut self, min: V3, max: V3, material: f32, collision: bool) {
        if collision {
            self.solids.push(Solid { min, max });
        }
        let [x, y, z] = min;
        let [a, b, c] = max;
        for (normal, corners, size) in [
            (
                [0., 0., 1.],
                [[x, y, c], [a, y, c], [a, b, c], [x, b, c]],
                [a - x, b - y],
            ),
            (
                [0., 0., -1.],
                [[a, y, z], [x, y, z], [x, b, z], [a, b, z]],
                [a - x, b - y],
            ),
            (
                [1., 0., 0.],
                [[a, y, c], [a, y, z], [a, b, z], [a, b, c]],
                [c - z, b - y],
            ),
            (
                [-1., 0., 0.],
                [[x, y, z], [x, y, c], [x, b, c], [x, b, z]],
                [c - z, b - y],
            ),
            (
                [0., 1., 0.],
                [[x, b, c], [a, b, c], [a, b, z], [x, b, z]],
                [a - x, c - z],
            ),
            (
                [0., -1., 0.],
                [[x, y, z], [a, y, z], [a, y, c], [x, y, c]],
                [a - x, c - z],
            ),
        ] {
            let base = u16::try_from(self.vertices.len()).expect("mesh exceeds u16 index budget");
            for (position, uv) in
                corners
                    .into_iter()
                    .zip([[0., 0.], [size[0], 0.], size, [0., size[1]]])
            {
                self.vertices.push(Vertex {
                    position,
                    normal,
                    uv,
                    material,
                });
            }
            self.indices
                .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }

    pub fn can_stand(&self, x: f32, z: f32) -> bool {
        !self.solids.iter().any(|s| s.blocks(x, z, 0.23))
    }
}

fn hash(x: u32, y: u32, seed: u32) -> u32 {
    let mut n = x.wrapping_mul(374761393) ^ y.wrapping_mul(668265263) ^ seed;
    n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    n ^ (n >> 16)
}

pub fn texture_atlas() -> Vec<u8> {
    let mut bytes = vec![0; TILE * TILE * TILES * 4];
    for tile in 0..TILES {
        for y in 0..TILE {
            for x in 0..TILE {
                let noise = (hash(x as u32, y as u32, 0x4b4b + tile as u32) & 15) as i32 - 7;
                let edge = x < 3 || y < 3 || x > 124 || y > 124;
                let seam = x == 4 || y == 4 || x == 123 || y == 123;
                let bolt = [(9, 9), (118, 9), (9, 118), (118, 118)]
                    .iter()
                    .any(|(bx, by)| (x as i32 - bx).pow(2) + (y as i32 - by).pow(2) < 9);
                let (base, tint): (i32, [f32; 3]) = match tile {
                    0 => (
                        if edge {
                            25
                        } else if seam {
                            95
                        } else if bolt {
                            140
                        } else {
                            76
                        },
                        [0.78, 0.88, 1.],
                    ),
                    1 => (
                        if edge {
                            22
                        } else if (x + y) % 16 < 2 {
                            53
                        } else {
                            84
                        },
                        [0.72, 0.78, 0.82],
                    ),
                    2 => (
                        if edge {
                            19
                        } else if y % 16 < 3 {
                            33
                        } else if bolt {
                            115
                        } else {
                            61
                        },
                        [0.85, 0.89, 0.93],
                    ),
                    _ => (if x % 32 < 3 { 110 } else { 220 }, [0.15, 0.82, 1.]),
                };
                let i = (y * TILE * TILES + tile * TILE + x) * 4;
                for c in 0..3 {
                    bytes[i + c] = ((base + noise).clamp(0, 255) as f32 * tint[c]) as u8;
                }
                bytes[i + 3] = 255;
            }
        }
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn room_has_valid_mesh_and_walkable_spawn() {
        let r = Room::generate();
        assert!(r.can_stand(0., 3.5));
        assert!(!r.can_stand(0., -12.));
        assert!(!r.can_stand(6., 0.));
        assert!(r.can_stand(2.5, -12.));
        assert!(r.indices.iter().all(|&i| (i as usize) < r.vertices.len()));
        for v in &r.vertices {
            assert!(v.position.iter().all(|x| x.is_finite()));
        }
    }
    #[test]
    fn atlas_is_reproducible_and_opaque() {
        let a = texture_atlas();
        assert_eq!(a, texture_atlas());
        assert_eq!(a.len(), TILE * TILE * TILES * 4);
        assert!(a.as_chunks::<4>().0.iter().all(|p| p[3] == 255));
        assert_ne!(&a[..TILE * 4], &a[TILE * 4..TILE * 8]);
    }
}
