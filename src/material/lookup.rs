//! Original 64² cube normalizer and 32³ attenuation texture texels.

pub fn normalizer() -> [Vec<u8>; 6] {
    const FACES: [[[f32; 3]; 2]; 6] = [
        [[0., 0., -1.], [0., 1., 0.]],
        [[0., 0., 1.], [0., 1., 0.]],
        [[1., 0., 0.], [0., 0., -1.]],
        [[1., 0., 0.], [0., 0., 1.]],
        [[1., 0., 0.], [0., 1., 0.]],
        [[-1., 0., 0.], [0., 1., 0.]],
    ];
    std::array::from_fn(|face| {
        let [a, b] = FACES[face];
        let cross = [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ];
        let mut out = Vec::with_capacity(64 * 64 * 4);
        for y in 0..64 {
            for x in 0..64 {
                let mut v = cross.map(|v| v * 31.5);
                for i in 0..3 {
                    v[i] += a[i] * (x as f32 - 31.5);
                    v[i] += b[i] * (-(y as f32) + 31.5);
                }
                let scale = 1. / (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
                out.extend(v.map(|v| (128. + v * scale * 127.).floor() as u8));
                out.push(0);
            }
        }
        out
    })
}

/// Rows are laid out as consecutive z slices, suitable for shader atlas sampling.
pub fn attenuation() -> Vec<u8> {
    let mut out = Vec::with_capacity(32 * 32 * 32 * 4);
    let scale = 2.0f32 / 30.;
    for z in 0..32 {
        let vz = (z as f32 - 16.) * scale;
        for y in 0..32 {
            let vy = (y as f32 - 16.) * scale;
            for x in 0..32 {
                let vx = (x as f32 - 16.) * scale;
                let a = ((1. - (vx * vx + vy * vy + vz * vz)).max(0.) * 255.).floor() as u8;
                out.extend([a; 4]);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn all_lookup_texels_match_original_cpp() {
        let expected = include_bytes!("../../tests/fixtures/lighting-lookup.bin");
        let mut actual: Vec<_> = super::normalizer().into_iter().flatten().collect();
        actual.extend(super::attenuation());
        assert_eq!(&actual[..], &expected[..]);
    }
}
