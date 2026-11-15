//! Original MonsterMagnetAI, preserving registration/pair order and RNG use.
use super::monster_registry::Handle;
use crate::random::Random;
fn dot(v: [f32; 4]) -> f32 {
    v[0] * v[0] + v[1] * v[1] + v[2] * v[2]
}
pub fn apply(monsters: &[Handle], rng: &mut Random) {
    let mut active = [0; 256];
    let mut count = 0;
    for (i, actor) in monsters.iter().enumerate() {
        let mut actor = actor.borrow_mut();
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
            let position_a = monsters[a].borrow().body.position;
            let position_b = monsters[b].borrow().body.position;
            let mut v = [
                position_a[0] - position_b[0],
                position_a[1] - position_b[1],
                position_a[2] - position_b[2],
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
                {
                    let mut actor = monsters[a].borrow_mut();
                    for (axis, value) in v[..3].iter().enumerate() {
                        actor.magnet_force[axis] += value * (distance * 0.001);
                    }
                }
                {
                    let mut actor = monsters[b].borrow_mut();
                    for (axis, value) in v[..3].iter().enumerate() {
                        actor.magnet_force[axis] += value * (distance * -0.001);
                    }
                }
            }
        }
    }
}
