//! Original KKriegerMonster::Hit state changes.
#[derive(Clone, Debug)]
pub struct Health {
    pub life: i32,
    pub armor: i32,
    pub state: i32,
    pub kill_switch: i32,
}
impl Health {
    pub fn hit(&mut self, hits: i32, switches: &mut [u8; 256]) -> Result<(), &'static str> {
        if self.life > 0 {
            if self.state == 3 {
                self.state = 4;
            }
            let damage = if hits > self.armor {
                hits.wrapping_sub(self.armor).wrapping_add(self.armor / 4)
            } else {
                hits / 4
            };
            self.life = self.life.wrapping_sub(damage);
            if self.life <= 0 {
                self.life = 0;
                if self.kill_switch > 0 {
                    let value = switches
                        .get_mut(self.kill_switch as usize)
                        .ok_or("monster kill switch out of range")?;
                    *value = value.wrapping_add(1);
                }
            }
        }
        Ok(())
    }
}
