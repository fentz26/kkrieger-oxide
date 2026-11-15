//! Original MonsterMagnetAI, preserving registration/pair order and RNG use.
use super::monster_actor::Actor;
use crate::random::Random;
fn dot(v: [f32; 4]) -> f32 {
    v[0] * v[0] + v[1] * v[1] + v[2] * v[2]
}
pub fn apply(monsters: &mut [&mut Actor], rng: &mut Random) {
    let mut active = [0; 256];
    let mut count = 0;
    for (i, actor) in monsters.iter_mut().enumerate() {
        actor.magnet_force = [0.; 4];
        if actor.health.state == 4 && count < 256 {
            active[count] = i;
            count += 1;
        }
    }
    for i in 0..count.saturating_sub(1) {
        for j in i + 1..count {
            let a = active[i];
            let b = active[j];
            let mut v = [
                monsters[a].body.position[0] - monsters[b].body.position[0],
                monsters[a].body.position[1] - monsters[b].body.position[1],
                monsters[a].body.position[2] - monsters[b].body.position[2],
                0.,
            ];
            let squared = dot(v);
            if squared < 1. {
                let distance = (1. - f64::from(squared).sqrt()) as f32;
                if distance > 0.99 {
                    loop {
                        for x in &mut v[..3] {
                            *x = rng.float() * 2. - 1.;
                        }
                        if dot(v) <= 1. {
                            break;
                        }
                    }
                } else {
                    let scale = (1. / f64::from(squared).sqrt()) as f32;
                    for x in &mut v[..3] {
                        *x *= scale;
                    }
                }
                let (left, right) = monsters.split_at_mut(b);
                for (axis, value) in v[..3].iter().enumerate() {
                    left[a].magnet_force[axis] += value * (distance * 0.001);
                    right[0].magnet_force[axis] += value * (distance * -0.001);
                }
            }
        }
    }
}
