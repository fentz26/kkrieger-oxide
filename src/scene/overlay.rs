//! Original player IPP render-target pool and master viewport bookkeeping.
pub const SIZES: usize = 4;
pub const PER_SIZE: usize = 3;
pub const COUNT: usize = SIZES * PER_SIZE;
pub const SCREEN: usize = COUNT;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Viewport {
    pub screen: i32,
    pub render_target: i32,
    pub window: [i32; 4],
}
#[derive(Clone, Debug)]
pub struct Target {
    pub owner: Option<usize>,
    pub size: usize,
    pub references: i32,
    pub scale_uv: [f32; 2],
    pub texture: Option<i32>,
}
pub struct Pool {
    pub targets: [Target; COUNT + 1],
    pub master: Viewport,
    pub last_output: Option<usize>,
}
impl Pool {
    pub fn new(master: Viewport, textures: [i32; COUNT]) -> Self {
        let mut pool = Self {
            targets: std::array::from_fn(|i| Target {
                owner: None,
                size: if i == SCREEN { SIZES } else { i / PER_SIZE },
                references: 0,
                scale_uv: if i == SCREEN { [0.; 2] } else { [1.; 2] },
                texture: textures.get(i).copied(),
            }),
            master,
            last_output: None,
        };
        pool.set_master(master);
        pool
    }
    /// Material unbinding belongs to the renderer; this resets allocation state.
    pub fn reset(&mut self) {
        for target in &mut self.targets[..COUNT] {
            target.references = 0;
        }
        self.last_output = None;
    }
    pub fn get(&self, size: usize, id: usize) -> Result<usize, &'static str> {
        if size >= SIZES {
            return Ok(SCREEN);
        }
        if id >= PER_SIZE {
            return Err("overlay target index out of range");
        }
        Ok(size * PER_SIZE + id)
    }
    pub fn allocate(&mut self, owner: Option<usize>, size: usize, count: i32) -> usize {
        for (index, target) in self.targets[..COUNT].iter_mut().enumerate() {
            if target.references == 0 && target.size == size {
                target.references = count;
                target.owner = owner;
                return index;
            }
        }
        SCREEN
    }
    pub fn find(&self, owner: Option<usize>) -> Option<usize> {
        owner?;
        self.targets[..COUNT]
            .iter()
            .position(|target| target.references != 0 && target.owner == owner)
    }
    pub fn free(&mut self, index: usize) -> Result<(), &'static str> {
        let target = self
            .targets
            .get_mut(index)
            .ok_or("overlay target out of range")?;
        if target.size < SIZES {
            target.references = target.references.wrapping_sub(1);
        }
        Ok(())
    }
    pub fn viewport(&self, index: usize) -> Result<Viewport, &'static str> {
        let target = self
            .targets
            .get(index)
            .ok_or("overlay target out of range")?;
        if target.size >= SIZES {
            return Ok(self.master);
        }
        let [width, height] = [[16, 16], [512, 256], [16, 16], [1024, 512]][target.size];
        let mut viewport = Viewport {
            screen: -1,
            render_target: target.texture.ok_or("overlay target has no texture")?,
            window: [0, 0, width, height],
        };
        if target.size == SIZES - 1 {
            viewport.window[2] =
                (self.master.window[2].wrapping_sub(self.master.window[0])).min(1024);
            viewport.window[3] =
                (self.master.window[3].wrapping_sub(self.master.window[1])).min(512);
        }
        Ok(viewport)
    }
    pub fn set_master(&mut self, viewport: Viewport) {
        self.master = viewport;
        let scale = [
            (viewport.window[2].wrapping_sub(viewport.window[0]) as f32 / 1024.).min(1.),
            (viewport.window[3].wrapping_sub(viewport.window[1]) as f32 / 512.).min(1.),
        ];
        for target in &mut self.targets[..COUNT] {
            if target.size == SIZES - 1 {
                target.scale_uv = scale;
            }
        }
    }
}
