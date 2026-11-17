//! Original KKriegerGame::OnOptionsChanged device settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub resolution: [u32; 2],
    pub usage_mask: u32,
}
impl Settings {
    pub fn read(switches: &[u8; 256]) -> Result<Self, &'static str> {
        let resolution = *[[640, 480], [800, 600], [1024, 768], [1280, 1024]]
            .get(usize::from(switches[8]))
            .ok_or("resolution option out of range")?;
        let usage_mask = if switches[10] != 0 {
            u32::MAX
        } else {
            !(1u32 << 3)
        };
        Ok(Self {
            resolution,
            usage_mask,
        })
    }
}
