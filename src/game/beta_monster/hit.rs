//! Released Particle::Hit final event-state transition. Hit-effect allocation
//! and dispatch must run before this operation when effect slots are present.

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
