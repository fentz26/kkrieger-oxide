//! Original player IPP target management and postprocessing draw commands.
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

/// sVertexDouble position, zero color word, and two UV pairs.
pub fn fx_quad(
    scale0: Option<[f32; 2]>,
    uv0: Option<[f32; 4]>,
    scale1: Option<[f32; 2]>,
    uv1: Option<[f32; 4]>,
) -> [[f32; 8]; 4] {
    let [s0, s1] = [scale0.unwrap_or([1.; 2]), scale1.unwrap_or([1.; 2])];
    let [uv0, uv1] = [
        uv0.unwrap_or([0., 0., 1., 1.]),
        uv1.unwrap_or([0., 0., 1., 1.]),
    ];
    std::array::from_fn(|i| {
        let j = i ^ (i >> 1);
        let x = usize::from(j & 1 != 0);
        let y = usize::from(j & 2 != 0);
        [
            if x != 0 { 1. } else { -1. },
            if y != 0 { -1. } else { 1. },
            1.,
            0.,
            uv0[x * 2] * s0[0],
            uv0[1 + y * 2] * s0[1],
            uv1[x * 2] * s1[0],
            uv1[1 + y * 2] * s1[1],
        ]
    })
}
#[derive(Clone, Debug)]
pub struct Draw {
    pub viewport: Viewport,
    pub environment: super::camera::Environment,
    pub material: usize,
    pub texture: i32,
    pub texture1: Option<i32>,
    pub color: u32,
    pub color2: Option<u32>,
    pub vertices: [[f32; 8]; 4],
}
impl Pool {
    pub fn copy(
        &self,
        source: usize,
        destination: usize,
        mode: usize,
        color: u32,
        mut zoom: f32,
    ) -> Result<Option<Draw>, &'static str> {
        let source = self
            .targets
            .get(source)
            .ok_or("overlay source out of range")?;
        let Some(texture) = source.texture else {
            return Ok(None);
        };
        let viewport = self.viewport(destination)?;
        let material = 27usize
            .checked_add(mode)
            .filter(|&index| index < 36)
            .ok_or("overlay copy material out of range")?;
        zoom *= 0.5;
        let uv = [0.5 - zoom, 0.5 - zoom, 0.5 + zoom, 0.5 + zoom];
        Ok(Some(Draw {
            viewport,
            environment: super::camera::Environment {
                orthogonal: 1,
                ..Default::default()
            },
            material,
            texture,
            texture1: None,
            color,
            color2: None,
            vertices: fx_quad(Some(source.scale_uv), Some(uv), None, None),
        }))
    }
    /// Exec_IPP_Copy after its input operators have executed. Source references
    /// are intentionally retained; this operator does not call Free(in).
    pub fn copy_operator(
        &mut self,
        owner: usize,
        mut size: usize,
        color: u32,
        zoom: f32,
        count: i32,
    ) -> Result<Option<Draw>, &'static str> {
        let Some(source) = self.last_output else {
            return Ok(None);
        };
        if size == SIZES + 1 {
            size = self
                .targets
                .get(source)
                .ok_or("overlay source out of range")?
                .size;
        }
        let (destination, draw) = if let Some(found) = self.find(Some(owner)) {
            (found, None)
        } else {
            let destination = self.allocate(Some(owner), size, count);
            let draw = self.copy(source, destination, 0, color, zoom)?;
            (destination, draw)
        };
        self.last_output = Some(destination);
        Ok(draw)
    }
}

impl Pool {
    /// Color differs from Copy: a cached output skips input execution, and a
    /// newly rendered output consumes one input reference.
    pub fn color_operator(
        &mut self,
        owner: usize,
        mut size: usize,
        color: u32,
        operation: usize,
        amplify: u32,
        count: i32,
    ) -> Result<Option<Draw>, &'static str> {
        if let Some(cached) = self.find(Some(owner)) {
            self.last_output = Some(cached);
            return Ok(None);
        }
        let Some(source) = self.last_output else {
            return Ok(None);
        };
        let input = self
            .targets
            .get(source)
            .ok_or("overlay source out of range")?;
        let Some(texture) = input.texture else {
            return Ok(None);
        };
        let scale = input.scale_uv;
        if size == SIZES + 1 {
            size = input.size;
        }
        let material = 8usize
            .checked_add(operation)
            .filter(|&index| index < 36)
            .ok_or("overlay color material out of range")?;
        let destination = self.allocate(Some(owner), size, count);
        let draw = Draw {
            viewport: self.viewport(destination)?,
            environment: super::camera::Environment {
                orthogonal: 1,
                ..Default::default()
            },
            material,
            texture,
            texture1: None,
            color,
            color2: Some(amplify),
            vertices: fx_quad(Some(scale), None, None, None),
        };
        self.free(source)?;
        self.last_output = Some(destination);
        Ok(Some(draw))
    }
}

pub struct Merge {
    pub size: usize,
    pub operation: usize,
    pub alpha: i32,
    pub amplify: u32,
    pub count: i32,
}
impl Pool {
    /// Called after both input operators have produced texture targets.
    pub fn merge_operator(
        &mut self,
        owner: usize,
        sources: [usize; 2],
        parameters: &Merge,
    ) -> Result<Option<Draw>, &'static str> {
        if let Some(cached) = self.find(Some(owner)) {
            self.last_output = Some(cached);
            return Ok(None);
        }
        let mut textures = [0; 2];
        let mut scales = [[0.; 2]; 2];
        for (i, source) in sources.into_iter().enumerate() {
            let target = self
                .targets
                .get(source)
                .ok_or("overlay source out of range")?;
            let Some(texture) = target.texture else {
                return Ok(None);
            };
            textures[i] = texture;
            scales[i] = target.scale_uv;
        }
        let size = if parameters.size == SIZES + 1 {
            self.targets[sources[0]].size
        } else {
            parameters.size
        };
        let material = 16usize
            .checked_add(parameters.operation)
            .filter(|&index| index < 36)
            .ok_or("overlay merge material out of range")?;
        let output = self.allocate(Some(owner), size, parameters.count);
        let draw = Draw {
            viewport: self.viewport(output)?,
            environment: super::camera::Environment {
                orthogonal: 1,
                ..Default::default()
            },
            material,
            texture: textures[0],
            texture1: Some(textures[1]),
            color: (parameters.alpha as u32).wrapping_shl(24),
            color2: Some(parameters.amplify),
            vertices: fx_quad(Some(scales[0]), None, Some(scales[1]), None),
        };
        for source in sources {
            self.free(source)?;
        }
        self.last_output = Some(output);
        Ok(Some(draw))
    }
}
