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

#[derive(Clone, Debug)]
pub struct Shot {
    pub kind: u32,
    pub event: Option<[u32; 45]>,
}
/// Registry-owned shot events and pending effects. Operator slots are indexed
/// by environment/monster target, then weapon kind 8..15.
pub struct HitRegistry {
    pub shots: std::collections::BTreeMap<usize, Shot>,
    pub operators: [[u32; 8]; 2],
    pub flags: [u32; 256],
    pub time: i32,
    pub pending: Vec<[u32; 45]>,
    /// DynamicEffects cannot return errors; callers must inspect this field.
    pub error: Option<&'static str>,
}
impl Default for HitRegistry {
    fn default() -> Self {
        Self {
            shots: Default::default(),
            operators: [[0; 8]; 2],
            flags: [0; 256],
            time: 0,
            pending: Vec::new(),
            error: None,
        }
    }
}
impl HitRegistry {
    pub fn first_particle(
        &self,
        id: usize,
    ) -> Result<super::subtractive::FirstParticle, &'static str> {
        let shot = self.shots.get(&id).ok_or("shot identity missing")?;
        Ok(super::subtractive::FirstParticle {
            id,
            kind: shot.kind,
            shot_select: shot.event.as_ref().map(|event| event[12] as i32),
        })
    }
    fn hit(
        &mut self,
        contact: super::subtractive::ParticleContact,
        select: &mut Option<i32>,
    ) -> Result<(), &'static str> {
        let shot = self
            .shots
            .get_mut(&contact.particle)
            .ok_or("shot identity missing")?;
        if shot.kind != contact.kind {
            return Err("shot kind differs from registry");
        }
        if shot.event.as_ref().map(|event| event[12] as i32) != *select {
            return Err("shot selection differs from registry");
        }
        let flags = *self
            .flags
            .get(shot.kind as usize)
            .ok_or("shot kind out of range")?;
        let operator = shot
            .kind
            .checked_sub(8)
            .and_then(|kind| self.operators[usize::from(contact.monster)].get(kind as usize))
            .copied()
            .unwrap_or(0);
        let effect = apply_hit(
            shot.event.as_mut(),
            Hit {
                kind: shot.kind as i32,
                reflect: flags & 2 != 0,
                monster: contact.monster,
                time: self.time,
                operator,
                position: contact.position,
                normal: contact.plane,
            },
        );
        *select = shot.event.as_ref().map(|event| event[12] as i32);
        if let Some(effect) = effect {
            self.pending.push(effect);
        }
        Ok(())
    }
}
impl super::subtractive::DynamicEffects for HitRegistry {
    fn particle_hit(
        &mut self,
        contact: super::subtractive::ParticleContact,
        select: &mut Option<i32>,
    ) {
        if let Err(error) = self.hit(contact, select) {
            self.error.get_or_insert(error);
        }
    }
}
