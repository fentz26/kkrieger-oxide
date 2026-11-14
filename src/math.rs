pub type V3 = [f32; 3];
pub type Mat4 = [f32; 16];

pub fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub fn normalize(v: V3) -> V3 {
    let len = dot(v, v).sqrt().max(0.00001);
    [v[0] / len, v[1] / len, v[2] / len]
}
pub fn mul(a: Mat4, b: Mat4) -> Mat4 {
    let mut out = [0.; 16];
    for c in 0..4 {
        for r in 0..4 {
            for k in 0..4 {
                out[c * 4 + r] += a[k * 4 + r] * b[c * 4 + k];
            }
        }
    }
    out
}
pub fn perspective(aspect: f32) -> Mat4 {
    let f = 1. / (70_f32.to_radians() / 2.).tan();
    let (near, far) = (0.04, 80.);
    [
        f / aspect,
        0.,
        0.,
        0.,
        0.,
        f,
        0.,
        0.,
        0.,
        0.,
        (far + near) / (near - far),
        -1.,
        0.,
        0.,
        2. * far * near / (near - far),
        0.,
    ]
}
pub fn view(eye: V3, yaw: f32, pitch: f32) -> Mat4 {
    let forward = [
        yaw.sin() * pitch.cos(),
        pitch.sin(),
        -yaw.cos() * pitch.cos(),
    ];
    let right = normalize(cross(forward, [0., 1., 0.]));
    let up = cross(right, forward);
    [
        right[0],
        up[0],
        -forward[0],
        0.,
        right[1],
        up[1],
        -forward[1],
        0.,
        right[2],
        up[2],
        -forward[2],
        0.,
        -dot(right, eye),
        -dot(up, eye),
        dot(forward, eye),
        1.,
    ]
}
