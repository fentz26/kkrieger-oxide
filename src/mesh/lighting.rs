use super::{Mesh, euler_matrix};

fn color(argb: u32, amplify: f32) -> [f32; 4] {
    [16, 8, 0, 24].map(|shift| ((argb >> shift) & 255) as f32 / 255. * amplify)
}

impl Mesh {
    pub fn light_slot(position: [f32; 3], argb: u32, amplify: f32, range: f32) -> Self {
        let mut mesh = Self::default();
        let vertex = mesh.add_vertex();
        mesh.light_slots.push(vertex);
        mesh.vertices[vertex].attributes[0] = [position[0], position[1], position[2], 1.];
        mesh.vertices[vertex].attributes[3] = color(argb, amplify);
        mesh.vertices[vertex].attributes[2] = [range, range * range, 1. / range, 0.];
        mesh
    }

    pub fn light_vertices(
        &mut self,
        position: [f32; 3],
        direction: [f32; 2],
        argb: u32,
        amplify: f32,
        range: f32,
        mode: i32,
    ) {
        self.all_to_selection(true, 7);
        self.need_all_normals();
        let matrix = euler_matrix([
            direction[0] * std::f32::consts::TAU,
            0.,
            direction[1] * std::f32::consts::TAU,
        ]);
        let direction = [matrix[0][1], matrix[1][1], matrix[2][1]];
        let light = color(argb, amplify);
        let squared_range = range * range;
        let inverse_range = 1. / range;
        for i in 0..self.vertices.len() {
            let attributes = self.vertices[i].attributes;
            let normal = attributes[1];
            match mode {
                0 => {
                    let intensity = direction[0] * normal[0]
                        + direction[1] * normal[1]
                        + direction[2] * normal[2];
                    if intensity > 0. {
                        for (axis, value) in light.iter().take(3).enumerate() {
                            self.vertices[i].attributes[3][axis] += value * intensity;
                        }
                    }
                }
                1 => {
                    let delta =
                        std::array::from_fn::<_, 3, _>(|axis| position[axis] - attributes[0][axis]);
                    let length = delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2];
                    if length < squared_range {
                        let intensity =
                            delta[0] * normal[0] + delta[1] * normal[1] + delta[2] * normal[2];
                        if intensity > 0. {
                            let scale = intensity * (1. / length.sqrt() - inverse_range);
                            for (axis, value) in light.iter().take(3).enumerate() {
                                self.vertices[i].attributes[3][axis] += value * scale;
                            }
                        }
                    }
                }
                2 => {
                    for &slot in &self.light_slots {
                        let source = self.vertices[slot].attributes;
                        let z = source[0][2] - attributes[0][2];
                        let mut length = z * z;
                        if length < source[2][1] {
                            let x = source[0][0] - attributes[0][0];
                            length += x * x;
                            let y = source[0][1] - attributes[0][1];
                            length += y * y;
                            if length < source[2][1] {
                                let intensity = x * normal[0] + y * normal[1] + z * normal[2];
                                if intensity > 0. {
                                    let scale = intensity * (1. / length.sqrt() - source[2][2]);
                                    for (axis, value) in source[3].iter().take(3).enumerate() {
                                        self.vertices[i].attributes[3][axis] += value * scale;
                                    }
                                }
                            }
                        }
                    }
                }
                3 => {
                    for (axis, value) in light.iter().take(3).enumerate() {
                        self.vertices[i].attributes[3][axis] += value;
                    }
                }
                _ => {}
            }
        }
    }
}
