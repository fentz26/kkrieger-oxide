//! Released Particle::Hit event construction and shot-state transition.
//! Event identities and scheduler insertion remain owned by the caller.

/// Apply the tail shared by effect-producing and no-effect paths. Event words
/// use the released 180-byte layout, rather than the development Event layout.
pub fn finish_hit(event: Option<&mut [u32; 45]>, reflect: bool, monster: bool, time: i32) {
    let Some(event) = event else { return };
    if !reflect || monster {
        event[12] = 1; // Select
        event[2] = 0; // Start (released Event::Stop)
        event[3] = 1; // End
    }
    event[5] = time as u32; // ShotTime also advances on reflection.
}

#[derive(Clone, Copy, Debug)]
pub struct Hit {
    pub kind: i32,
    pub reflect: bool,
    pub monster: bool,
    pub time: i32,
    /// Effect identity already selected from the kind/monster slot table.
    pub operator: u32,
    pub position: [f32; 4],
    pub normal: [f32; 4],
}
/// Construct a released effect event in fixed storage. The caller owns insertion
/// into the event scheduler; null operators and the original timing gate skip it.
pub fn apply_hit(event: Option<&mut [u32; 45]>, hit: Hit) -> Option<[u32; 45]> {
    let event = event?;
    let mut effect = None;
    if (8..16).contains(&hit.kind)
        && event[2] as i32 > 0
        && event[12] == 0
        && hit.operator != 0
        && (hit.monster || (event[5] as i32).wrapping_add(10) < hit.time)
    {
        event[10] = 5f32.to_bits();
        let mut created = [0; 45];
        created[0] = hit.operator;
        created[21..25].copy_from_slice(&hit.position.map(f32::to_bits));
        let [x, y, z, _] = hit.normal.map(f64::from);
        let horizontal_squared = super::add(super::mul(x, x), super::mul(z, z));
        let root = horizontal_squared.sqrt();
        let horizontal = super::rounded_arithmetic(root, (-root).mul_add(root, horizontal_squared));
        // Original FPATAN operands: atan2(-normal.y, horizontal length),
        // then atan2(normal.x, normal.z). The scale is a stored f64 constant.
        const TURNS: f64 = 0.15915493866300567;
        created[17] = super::down(super::mul((-y).atan2(horizontal), TURNS)).to_bits();
        created[18] = super::down(super::mul(x.atan2(z), TURNS)).to_bits();
        effect = Some(created);
    }
    finish_hit(Some(event), hit.reflect, hit.monster, hit.time);
    effect
}
