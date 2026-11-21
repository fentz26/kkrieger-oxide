//! Original player IPP target management and postprocessing draw commands.
pub const SIZES: usize = 4;
pub const PER_SIZE: usize = 3;
pub const COUNT: usize = SIZES * PER_SIZE;
pub const SCREEN: usize = COUNT;
pub const DIMENSIONS: [[i32; 2]; SIZES] = [[16, 16], [512, 256], [16, 16], [1024, 512]];
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
    released: bool,
    pub targets: [Target; COUNT + 1],
    pub master: Viewport,
    pub last_output: Option<usize>,
}
impl Pool {
    pub fn new(master: Viewport, textures: [i32; COUNT]) -> Self {
        Self::new_for_layout(master, textures, crate::kx::Layout::Current)
    }
    pub fn new_for_layout(
        master: Viewport,
        textures: [i32; COUNT],
        layout: crate::kx::Layout,
    ) -> Self {
        let mut pool = Self {
            released: layout == crate::kx::Layout::Beta2004,
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
    /// Exec_IPP_Select adjusts even screen references; Free deliberately does not.
    pub fn select(&mut self, count: i32) -> Result<(), &'static str> {
        if let Some(output) = self.last_output {
            let target = self
                .targets
                .get_mut(output)
                .ok_or("overlay output out of range")?;
            target.references = target.references.wrapping_add(count.wrapping_sub(1));
        }
        Ok(())
    }
    /// Released PrepareViewport keeps full texture extent; its screen fallback
    /// is the original fixed 1024x768 letterbox, not the later master viewport.
    pub fn beta_viewport(&self, index: usize) -> Result<Viewport, &'static str> {
        let target = self
            .targets
            .get(index)
            .ok_or("overlay target out of range")?;
        if target.size >= SIZES {
            return Ok(Viewport {
                screen: 0,
                render_target: -1,
                window: [0, 128, 1024, 640],
            });
        }
        let [width, height] = DIMENSIONS[target.size];
        Ok(Viewport {
            screen: -1,
            render_target: target.texture.ok_or("overlay target has no texture")?,
            window: [0, 0, width, height],
        })
    }
    pub fn viewport(&self, index: usize) -> Result<Viewport, &'static str> {
        let target = self
            .targets
            .get(index)
            .ok_or("overlay target out of range")?;
        if target.size >= SIZES {
            return Ok(self.master);
        }
        let [width, height] = DIMENSIONS[target.size];
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
/// Layout-aware postprocessing quad; viewport dimensions are destination pixels.
pub fn fx_quad_for_layout(
    layout: crate::kx::Layout,
    viewport: [i32; 2],
    scale0: Option<[f32; 2]>,
    uv0: Option<[f32; 4]>,
    scale1: Option<[f32; 2]>,
    uv1: Option<[f32; 4]>,
) -> [[f32; 8]; 4] {
    let mut vertices = fx_quad(scale0, uv0, scale1, uv1);
    if layout == crate::kx::Layout::Beta2004 {
        for vertex in &mut vertices {
            vertex[0] = if vertex[0] < 0. {
                -0.5
            } else {
                crate::game::beta_monster::down(f64::from(viewport[0]) - 0.5)
            };
            vertex[1] = if vertex[1] > 0. {
                -0.5
            } else {
                crate::game::beta_monster::down(f64::from(viewport[1]) - 0.5)
            };
        }
    }
    vertices
}
#[derive(Clone, Debug)]
pub struct Draw {
    pub viewport: Viewport,
    pub environment: super::camera::Environment,
    pub material: usize,
    pub texture: i32,
    pub texture1: Option<i32>,
    pub texture2: Option<i32>,
    pub color: u32,
    pub color2: Option<u32>,
    pub vertices: [[f32; 8]; 4],
}
impl Pool {
    fn environment(&self) -> super::camera::Environment {
        super::camera::Environment {
            orthogonal: if self.released { 2 } else { 1 },
            released_projection: self.released,
            ..Default::default()
        }
    }
    fn quad(
        &self,
        viewport: Viewport,
        scale0: Option<[f32; 2]>,
        uv0: Option<[f32; 4]>,
        scale1: Option<[f32; 2]>,
        uv1: Option<[f32; 4]>,
    ) -> [[f32; 8]; 4] {
        let [x0, y0, x1, y1] = viewport.window;
        fx_quad_for_layout(
            if self.released {
                crate::kx::Layout::Beta2004
            } else {
                crate::kx::Layout::Current
            },
            [x1.wrapping_sub(x0), y1.wrapping_sub(y0)],
            scale0,
            uv0,
            scale1,
            uv1,
        )
    }
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
            environment: self.environment(),
            material,
            texture,
            texture1: None,
            texture2: None,
            color,
            color2: None,
            vertices: self.quad(viewport, Some(source.scale_uv), Some(uv), None, None),
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
        let viewport = self.viewport(destination)?;
        let draw = Draw {
            viewport,
            environment: self.environment(),
            material,
            texture,
            texture1: None,
            texture2: None,
            color,
            color2: Some(amplify),
            vertices: self.quad(viewport, Some(scale), None, None, None),
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
        let viewport = self.viewport(output)?;
        let draw = Draw {
            viewport,
            environment: self.environment(),
            material,
            texture: textures[0],
            texture1: Some(textures[1]),
            texture2: None,
            color: (parameters.alpha as u32).wrapping_shl(24),
            color2: Some(parameters.amplify),
            vertices: self.quad(viewport, Some(scales[0]), None, Some(scales[1]), None),
        };
        for source in sources {
            self.free(source)?;
        }
        self.last_output = Some(output);
        Ok(Some(draw))
    }
}

impl Pool {
    /// Original mask draws (mask dot color) * B, then (1-mask dot color) * A.
    pub fn mask_operator(
        &mut self,
        owner: usize,
        sources: [usize; 3],
        mut size: usize,
        count: i32,
    ) -> Result<Vec<Draw>, &'static str> {
        if let Some(cached) = self.find(Some(owner)) {
            self.last_output = Some(cached);
            return Ok(vec![]);
        }
        let mut textures = [0; 3];
        let mut scales = [[0.; 2]; 3];
        for (index, source) in sources.into_iter().enumerate() {
            let target = self
                .targets
                .get(source)
                .ok_or("overlay source out of range")?;
            let Some(texture) = target.texture else {
                return Ok(vec![]);
            };
            textures[index] = texture;
            scales[index] = target.scale_uv;
        }
        if size == SIZES + 1 {
            size = self.targets[sources[0]].size;
        }
        let output = self.allocate(Some(owner), size, count);
        let viewport = self.viewport(output)?;
        let first = Draw {
            viewport,
            environment: self.environment(),
            material: 25,
            texture: textures[2],
            texture1: Some(textures[1]),
            texture2: None,
            color: 0x408040,
            color2: Some(u32::MAX),
            vertices: self.quad(viewport, Some(scales[2]), None, Some(scales[1]), None),
        };
        let second = Draw {
            viewport,
            environment: self.environment(),
            material: 26,
            texture: textures[2],
            texture1: None,
            texture2: Some(textures[0]),
            color: 0x408040,
            color2: Some(0xffffff),
            vertices: self.quad(viewport, Some(scales[2]), None, Some(scales[0]), None),
        };
        for source in sources {
            self.free(source)?;
        }
        self.last_output = Some(output);
        Ok(vec![first, second])
    }
}

pub struct Blend {
    pub size: usize,
    pub coordinate_mode: bool,
    pub amplify: f32,
    pub count: i32,
}
pub struct Blur {
    pub size: usize,
    pub radius: f32,
    pub amplify: f32,
    pub flags: u32,
    pub stages: i32,
    pub count: i32,
}
impl Pool {
    pub fn blend4x(
        &self,
        source: usize,
        destination: usize,
        rects: &mut [[f32; 4]; 4],
        coordinate_mode: bool,
        amplify: f32,
    ) -> Result<Vec<Draw>, &'static str> {
        let source = self
            .targets
            .get(source)
            .ok_or("overlay source out of range")?;
        let Some(texture) = source.texture else {
            return Ok(vec![]);
        };
        let viewport = self.viewport(destination)?;
        if coordinate_mode {
            let [width, height] = DIMENSIONS[source.size];
            for rect in rects.iter_mut() {
                for pair in 0..2 {
                    rect[pair * 2] = rect[pair * 2] / width as f32 + pair as f32;
                    rect[pair * 2 + 1] = rect[pair * 2 + 1] / height as f32 + pair as f32;
                }
            }
        }
        // Original x86 truncating float-to-int conversion, before sRange<int>.
        let scaled = amplify * 64.;
        let alpha = if (-2147483648.0..2147483648.0).contains(&scaled) {
            scaled as i32
        } else {
            i32::MIN
        };
        let color = (alpha.clamp(0, 255) as u32) << 24;
        Ok((0..2)
            .map(|pass| Draw {
                viewport,
                environment: self.environment(),
                material: 4 + pass,
                texture,
                texture1: Some(texture),
                texture2: None,
                color,
                color2: Some(u32::MAX),
                vertices: self.quad(
                    viewport,
                    Some(source.scale_uv),
                    Some(rects[pass * 2]),
                    Some(source.scale_uv),
                    Some(rects[pass * 2 + 1]),
                ),
            })
            .collect())
    }
    pub fn blend4x_operator(
        &mut self,
        owner: Option<usize>,
        source: Option<usize>,
        rects: &mut [[f32; 4]; 4],
        parameters: &Blend,
    ) -> Result<(Option<usize>, Vec<Draw>), &'static str> {
        let Some(source) = source else {
            return Ok((None, vec![]));
        };
        let size = if parameters.size == SIZES + 1 {
            self.targets
                .get(source)
                .ok_or("overlay source out of range")?
                .size
        } else {
            parameters.size
        };
        let output = self.allocate(owner, size, parameters.count);
        let draws = self.blend4x(
            source,
            output,
            rects,
            parameters.coordinate_mode,
            parameters.amplify,
        )?;
        self.free(source)?;
        Ok((Some(output), draws))
    }
    pub fn blur_operator(
        &mut self,
        owner: usize,
        parameters: &Blur,
    ) -> Result<Vec<Draw>, &'static str> {
        let (output, draws) = self.blur_op(Some(owner), self.last_output, parameters)?;
        self.last_output = output;
        Ok(draws)
    }
    fn blur_op(
        &mut self,
        owner: Option<usize>,
        mut input: Option<usize>,
        parameters: &Blur,
    ) -> Result<(Option<usize>, Vec<Draw>), &'static str> {
        if let Some(cached) = self.find(owner) {
            return Ok((Some(cached), vec![]));
        }
        let mut radius = parameters.radius;
        let mut count = 0;
        loop {
            radius *= 0.5;
            count += 1;
            if !(count < parameters.stages && radius > 0.5) {
                break;
            }
        }
        let kernels = [
            [-1., -1.],
            [1., -1.],
            [-1., 1.],
            [1., 1.],
            [0., -1.],
            [-1., 0.],
            [1., 0.],
            [0., 1.],
        ];
        let mut draws = vec![];
        while count > 0 {
            count -= 1;
            let mut rects = std::array::from_fn(|index| {
                let [x, y] = kernels[(parameters.flags as usize & 1) * 4 + index];
                let x = x * radius;
                let y = y * radius * 0.5;
                [x, y, x, y]
            });
            let stage = Blend {
                coordinate_mode: true,
                amplify: if count > 0 && parameters.flags & 2 == 0 {
                    1.
                } else {
                    parameters.amplify
                },
                size: if count > 0 && parameters.size == SIZES {
                    0
                } else {
                    parameters.size
                },
                count: if count > 0 { 1 } else { parameters.count },
            };
            let (output, stage_draws) = self.blend4x_operator(
                if count == 0 { owner } else { None },
                input,
                &mut rects,
                &stage,
            )?;
            input = output;
            draws.extend(stage_draws);
            radius *= 2.;
        }
        Ok((input, draws))
    }
}

impl Pool {
    pub fn sharpen_operator(
        &mut self,
        owner: usize,
        size: usize,
        radius: f32,
        amplify: u32,
        stages: i32,
        count: i32,
    ) -> Result<Vec<Draw>, &'static str> {
        if let Some(cached) = self.find(Some(owner)) {
            self.last_output = Some(cached);
            return Ok(vec![]);
        }
        let Some(input) = self.last_output else {
            return Ok(vec![]);
        };
        let target = self
            .targets
            .get_mut(input)
            .ok_or("overlay source out of range")?;
        if target.texture.is_none() {
            return Ok(vec![]);
        }
        target.references = target.references.wrapping_add(1);
        let (blurred, mut draws) = self.blur_op(
            None,
            Some(input),
            &Blur {
                size,
                radius,
                amplify: 1.,
                flags: 0,
                stages,
                count: 1,
            },
        )?;
        let blurred = blurred.ok_or("sharpen blur produced no target")?;
        let Some(texture) = self.targets[blurred].texture else {
            return Ok(draws);
        };
        let size = if size == SIZES + 1 {
            self.targets[input].size
        } else {
            size
        };
        let output = self.allocate(Some(owner), size, count);
        let viewport = self.viewport(output)?;
        draws.push(Draw {
            viewport,
            environment: self.environment(),
            material: 7,
            texture,
            texture1: self.targets[input].texture,
            texture2: None,
            color: 0x00ffffff,
            color2: Some(amplify),
            vertices: self.quad(
                viewport,
                Some(self.targets[blurred].scale_uv),
                None,
                Some(self.targets[input].scale_uv),
                None,
            ),
        });
        self.free(input)?;
        self.free(blurred)?;
        self.last_output = Some(output);
        Ok(draws)
    }
}
