//! Original static, shadow, sprite, and packed effect jobs on the GPU.
//! Mesh sprites and thick lines retain their original standard quad layout.
use crate::{
    material::{self, Program, State, uniforms::Environment},
    prepared_mesh::PreparedMesh,
};
use miniquad::*;

pub struct StaticJob {
    pub vertex: BufferId,
    pub index: BufferId,
    pub index_count: i32,
}
impl StaticJob {
    pub fn upload(
        ctx: &mut dyn RenderingBackend,
        mesh: &PreparedMesh,
        job: usize,
    ) -> Result<Self, &'static str> {
        let job = mesh.jobs.get(job).ok_or("mesh job index out of range")?;
        if job.program != Program::Static {
            return Err("requested mesh job requires a different original finalizer");
        }
        let vertices: Vec<_> = job
            .vertices
            .iter()
            .map(|&i| mesh.vertices[i].clone())
            .collect();
        if vertices.is_empty() || job.indices.is_empty() {
            return Err("static job has no triangles");
        }
        let vertex = ctx.new_buffer(
            BufferType::VertexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&vertices),
        );
        let index = if vertices.len() <= 65536 {
            let indices: Vec<_> = job.indices.iter().map(|&i| i as u16).collect();
            ctx.new_buffer(
                BufferType::IndexBuffer,
                BufferUsage::Immutable,
                BufferSource::slice(&indices),
            )
        } else {
            let indices: Vec<_> = job.indices.iter().map(|&i| i as u32).collect();
            ctx.new_buffer(
                BufferType::IndexBuffer,
                BufferUsage::Immutable,
                BufferSource::slice(&indices),
            )
        };
        Ok(Self {
            vertex,
            index,
            index_count: job.indices.len() as i32,
        })
    }
    pub fn delete(self, ctx: &mut dyn RenderingBackend) {
        ctx.delete_buffer(self.vertex);
        ctx.delete_buffer(self.index);
    }
}

pub struct Material {
    pub shader: ShaderId,
    pub pipeline: Pipeline,
    pub images: Vec<(String, material::shader::Image)>,
    shadow_mode: Option<bool>,
    effect_layout: bool,
    standard_layout: bool,
    double_layout: bool,
    depth_bits: u32,
}
impl Material {
    pub fn compile(
        ctx: &mut dyn RenderingBackend,
        state: &State,
        depth_bits: u32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::compile_kind(ctx, state, depth_bits, None, false, false, false)
    }
    pub fn compile_shadow(
        ctx: &mut dyn RenderingBackend,
        state: &State,
        depth_bits: u32,
        zfail: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        if state.words[0] & 0x40000 == 0 {
            return Err("shadow material is missing source extrusion flag".into());
        }
        Self::compile_kind(ctx, state, depth_bits, Some(zfail), false, false, false)
    }
    pub fn compile_effect(
        ctx: &mut dyn RenderingBackend,
        state: &State,
        depth_bits: u32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        if state.words[0] & 0x40000 != 0 {
            return Err("effect material cannot use shadow extrusion".into());
        }
        Self::compile_kind(ctx, state, depth_bits, None, true, false, false)
    }
    pub fn compile_standard(
        ctx: &mut dyn RenderingBackend,
        state: &State,
        depth_bits: u32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        if state.words[0] & 0x40000 != 0 {
            return Err("standard material cannot use shadow extrusion".into());
        }
        Self::compile_kind(ctx, state, depth_bits, None, false, true, false)
    }
    pub fn compile_double(
        ctx: &mut dyn RenderingBackend,
        state: &State,
        depth_bits: u32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        if state.words[0] & 0x40000 != 0 {
            return Err("double-UV material cannot use shadow extrusion".into());
        }
        Self::compile_kind(ctx, state, depth_bits, None, false, false, true)
    }
    fn compile_kind(
        ctx: &mut dyn RenderingBackend,
        state: &State,
        depth_bits: u32,
        shadow: Option<bool>,
        effect_layout: bool,
        standard_layout: bool,
        double_layout: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        if ctx.info().backend != Backend::OpenGl {
            return Err("original material rendering requires OpenGL".into());
        }
        let mut source = state.shader_sources()?;
        if effect_layout {
            // DeclTSpace3 supplies D3DCOLOR normal/tangent inputs. Miniquad's
            // Byte4 inputs retain byte magnitude and BGRA memory ordering.
            source.vertex = source.vertex.replace(
                "vec3 N=normal;vec3 T=tangent;",
                "vec3 N=normal.zyx/255.0;vec3 T=tangent.zyx/255.0;",
            );
        }
        let shader = ctx.new_shader(
            ShaderSource::Glsl {
                vertex: &source.vertex,
                fragment: &source.fragment,
            },
            ShaderMeta {
                images: source.images.iter().map(|(name, _)| name.clone()).collect(),
                uniforms: UniformBlockLayout {
                    uniforms: vec![
                        UniformDesc::new("vc", UniformType::Float4).array(24),
                        UniformDesc::new("pc", UniformType::Float4).array(8),
                        UniformDesc::new("fog_color", UniformType::Float4),
                    ],
                },
            },
        )?;
        let attributes = if shadow.is_some() {
            vec![VertexAttribute::new("position", VertexFormat::Float4)]
        } else if double_layout {
            vec![
                VertexAttribute::new("position", VertexFormat::Float3),
                VertexAttribute::new("color", VertexFormat::Byte4),
                VertexAttribute::new("uv0", VertexFormat::Float2),
                VertexAttribute::new("uv1", VertexFormat::Float2),
                VertexAttribute::new("_unused0", VertexFormat::Float1),
                VertexAttribute::new("_unused1", VertexFormat::Float1),
            ]
        } else if standard_layout {
            vec![
                VertexAttribute::new("position", VertexFormat::Float3),
                VertexAttribute::new("normal", VertexFormat::Float3),
                VertexAttribute::new("uv0", VertexFormat::Float2),
                // Miniquad allocates locations from this list's length. GLSL
                // drivers can retain holes for inactive declared attributes.
                VertexAttribute::new("_unused0", VertexFormat::Float1),
                VertexAttribute::new("_unused1", VertexFormat::Float1),
                VertexAttribute::new("_unused2", VertexFormat::Float1),
            ]
        } else if effect_layout {
            vec![
                VertexAttribute::new("position", VertexFormat::Float3),
                VertexAttribute::new("normal", VertexFormat::Byte4),
                VertexAttribute::new("tangent", VertexFormat::Byte4),
                VertexAttribute::new("color", VertexFormat::Byte4),
                VertexAttribute::new("uv0", VertexFormat::Float2),
            ]
        } else {
            vec![
                VertexAttribute::new("position", VertexFormat::Float3),
                VertexAttribute::new("normal", VertexFormat::Float3),
                VertexAttribute::new("tangent", VertexFormat::Float3),
                VertexAttribute::new("color", VertexFormat::Byte4),
                VertexAttribute::new("uv0", VertexFormat::Float2),
            ]
        };
        let mut pipeline_state = state.clone();
        if let Some(zfail) = shadow {
            pipeline_state.words[0] =
                (pipeline_state.words[0] & !0xf00006) | 0x500002 | if zfail { 0x800000 } else { 0 };
        }
        let layout = if standard_layout || double_layout {
            BufferLayout {
                stride: std::mem::size_of::<crate::mesh_sprite::Vertex>() as i32,
                ..Default::default()
            }
        } else {
            BufferLayout::default()
        };
        let pipeline = ctx.new_pipeline(
            &[layout],
            &attributes,
            shader,
            pipeline_state.pipeline_params(depth_bits),
        );
        Ok(Self {
            shader,
            pipeline,
            images: source.images,
            shadow_mode: shadow,
            effect_layout,
            standard_layout,
            double_layout,
            depth_bits,
        })
    }
    pub fn draw(
        &self,
        ctx: &mut dyn RenderingBackend,
        job: &StaticJob,
        images: &[TextureId],
        state: &State,
        environment: &Environment,
    ) -> Result<(), &'static str> {
        if self.shadow_mode.is_some()
            || self.effect_layout
            || self.standard_layout
            || self.double_layout
        {
            return Err("shadow material requires shadow vertex layout");
        }
        if images.len() != self.images.len() {
            return Err("material texture binding count mismatch");
        }
        ctx.apply_pipeline(&self.pipeline);
        apply_depth(state, self.depth_bits);
        ctx.apply_bindings_from_slice(&[job.vertex], job.index, images);
        let uniforms = raster_uniforms(state, environment);
        ctx.apply_uniforms(UniformsSource::table(&uniforms));
        ctx.draw(0, job.index_count, 1);
        Ok(())
    }
    pub fn draw_effect(
        &self,
        ctx: &mut dyn RenderingBackend,
        job: &EffectJob,
        images: &[TextureId],
        state: &State,
        environment: &Environment,
    ) -> Result<(), &'static str> {
        if !self.effect_layout || self.shadow_mode.is_some() {
            return Err("effect draw requires original packed TSPACE3 layout");
        }
        if images.len() != self.images.len() {
            return Err("material texture binding count mismatch");
        }
        if job.index_count == 0 {
            return Ok(());
        }
        ctx.apply_pipeline(&self.pipeline);
        apply_depth(state, self.depth_bits);
        ctx.apply_bindings_from_slice(&[job.vertex], job.index, images);
        ctx.apply_uniforms(UniformsSource::table(&raster_uniforms(state, environment)));
        ctx.draw(0, job.index_count, 1);
        Ok(())
    }
    pub fn draw_double(
        &self,
        ctx: &mut dyn RenderingBackend,
        job: &EffectJob,
        images: &[TextureId],
        state: &State,
        environment: &Environment,
    ) -> Result<(), &'static str> {
        if !self.double_layout {
            return Err("double-UV draw requires original double vertex layout");
        }
        if images.len() != self.images.len() {
            return Err("material texture binding count mismatch");
        }
        if job.index_count == 0 {
            return Ok(());
        }
        ctx.apply_pipeline(&self.pipeline);
        apply_depth(state, self.depth_bits);
        ctx.apply_bindings_from_slice(&[job.vertex], job.index, images);
        ctx.apply_uniforms(UniformsSource::table(&raster_uniforms(state, environment)));
        ctx.draw(0, job.index_count, 1);
        Ok(())
    }
    pub fn draw_standard(
        &self,
        ctx: &mut dyn RenderingBackend,
        job: &StandardJob,
        images: &[TextureId],
        state: &State,
        environment: &Environment,
    ) -> Result<(), &'static str> {
        if !self.standard_layout || self.shadow_mode.is_some() {
            return Err("standard draw requires original standard vertex layout");
        }
        if images.len() != self.images.len() {
            return Err("material texture binding count mismatch");
        }
        let job = &job.0;
        if job.index_count == 0 {
            return Ok(());
        }
        ctx.apply_pipeline(&self.pipeline);
        apply_depth(state, self.depth_bits);
        ctx.apply_bindings_from_slice(&[job.vertex], job.index, images);
        ctx.apply_uniforms(UniformsSource::table(&raster_uniforms(state, environment)));
        ctx.draw(0, job.index_count, 1);
        Ok(())
    }
    pub fn draw_shadow(
        &self,
        ctx: &mut dyn RenderingBackend,
        job: &ShadowJob,
        state: &State,
        environment: &Environment,
    ) -> Result<(), &'static str> {
        if self.shadow_mode.is_none() {
            return Err("shadow draw requires an original shadow material");
        }
        if job.index_count == 0 {
            return Ok(());
        }
        if self.shadow_mode != Some(job.zfail) {
            return Err("shadow cap mode does not match stencil pipeline");
        }
        if !self.images.is_empty() {
            return Err("original shadow shader must not sample textures");
        }
        ctx.apply_pipeline(&self.pipeline);
        apply_depth(state, self.depth_bits);
        ctx.apply_bindings_from_slice(&[job.vertex], job.index, &[]);
        ctx.apply_uniforms(UniformsSource::table(&raster_uniforms(state, environment)));
        ctx.draw(0, job.index_count, 1);
        Ok(())
    }
    pub fn delete(self, ctx: &mut dyn RenderingBackend) {
        ctx.delete_pipeline(self.pipeline);
        ctx.delete_shader(self.shader);
    }
}

pub struct ShadowJob {
    vertex: BufferId,
    index: BufferId,
    index_count: i32,
    vertex_count: usize,
    index_capacity: usize,
    big_indices: bool,
    zfail: bool,
}
impl ShadowJob {
    pub fn new(
        ctx: &mut dyn RenderingBackend,
        volume: &crate::shadow::Volume,
        index_capacity: usize,
    ) -> Result<Self, &'static str> {
        if volume.vertices.is_empty() {
            return Err("shadow job has no vertices");
        }
        let vertex = ctx.new_buffer(
            BufferType::VertexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&volume.vertices),
        );
        let big_indices = volume.vertices.len() >= 65536;
        let index_capacity = index_capacity.max(3);
        let index = if big_indices {
            ctx.new_buffer(
                BufferType::IndexBuffer,
                BufferUsage::Stream,
                BufferSource::empty::<u32>(index_capacity),
            )
        } else {
            ctx.new_buffer(
                BufferType::IndexBuffer,
                BufferUsage::Stream,
                BufferSource::empty::<u16>(index_capacity),
            )
        };
        Ok(Self {
            vertex,
            index,
            index_count: 0,
            vertex_count: volume.vertices.len(),
            index_capacity,
            big_indices,
            zfail: false,
        })
    }
    pub fn update(
        &mut self,
        ctx: &mut dyn RenderingBackend,
        volume: &crate::shadow::Volume,
        zfail: bool,
    ) -> Result<(), &'static str> {
        if volume.vertices.len() != self.vertex_count {
            return Err("shadow vertex count changed");
        }
        let indices = volume.draw_indices(zfail);
        if indices.len() > self.index_capacity || indices.iter().any(|&i| i >= self.vertex_count) {
            return Err("shadow index storage exceeded");
        }
        if !indices.is_empty() {
            if self.big_indices {
                let data: Vec<_> = indices.iter().map(|&i| i as u32).collect();
                ctx.buffer_update(self.index, BufferSource::slice(&data));
            } else {
                let data: Vec<_> = indices.iter().map(|&i| i as u16).collect();
                ctx.buffer_update(self.index, BufferSource::slice(&data));
            }
        }
        self.index_count = indices.len() as i32;
        self.zfail = zfail;
        Ok(())
    }
    pub fn delete(self, ctx: &mut dyn RenderingBackend) {
        ctx.delete_buffer(self.vertex);
        ctx.delete_buffer(self.index);
    }
}

// Miniquad 0.4.11 uses depth_write to enable GL_DEPTH_TEST and never applies
// glDepthMask or polygon bias. Original light/shadow passes need read-only depth.
fn apply_depth(state: &State, depth_bits: u32) {
    use miniquad::native::gl::*;
    let params = state.pipeline_params(depth_bits);
    unsafe {
        glEnable(GL_DEPTH_TEST);
        glDepthFunc(params.depth_test.into());
        glDepthMask(u8::from(params.depth_write));
        if let Some((factor, units)) = params.depth_write_offset {
            glEnable(GL_POLYGON_OFFSET_FILL);
            glPolygonOffset(factor, units);
        } else {
            glDisable(GL_POLYGON_OFFSET_FILL);
        }
    }
}

/// GPU resources retained across authored static paint jobs and frames.
/// Source allocations stay alive so cache keys cannot alias reused addresses.
pub struct PaintCache {
    depth_bits: u32,
    compilations: usize,
    shadow_context: crate::shadow::Context,
    shadow_geometry:
        std::collections::BTreeMap<(usize, usize), (std::rc::Rc<PreparedMesh>, ShadowJob)>,
    shadow_materials:
        std::collections::BTreeMap<(usize, bool), (std::rc::Rc<material::Recipe>, State, Material)>,
    geometry: std::collections::BTreeMap<(usize, usize), (std::rc::Rc<PreparedMesh>, StaticJob)>,
    materials: std::collections::BTreeMap<usize, (std::rc::Rc<material::Recipe>, State, Material)>,
}
impl PaintCache {
    pub fn new(depth_bits: u32) -> Self {
        Self {
            depth_bits,
            compilations: 0,
            shadow_context: Default::default(),
            shadow_geometry: Default::default(),
            shadow_materials: Default::default(),
            geometry: Default::default(),
            materials: Default::default(),
        }
    }
    pub fn compilation_count(&self) -> usize {
        self.compilations
    }
    pub fn resource_counts(&self) -> (usize, usize) {
        (self.geometry.len(), self.materials.len())
    }
    /// Submit a static geometry mapping produced by the scene paint planner.
    /// Texture resolution retains the recipe's original texture slot semantics.
    pub fn draw_static(
        &mut self,
        ctx: &mut dyn RenderingBackend,
        asset: &crate::scene::paint::Asset,
        paint: &crate::scene::paint::MeshPaint,
        environment: &Environment,
        mut texture: impl FnMut(
            &mut dyn RenderingBackend,
            &material::Recipe,
            material::shader::Image,
        ) -> Result<TextureId, Box<dyn std::error::Error>>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        use std::{collections::btree_map::Entry, rc::Rc};
        let pass = asset
            .materials
            .get(paint.material_slot)
            .and_then(|m| m.passes.get(paint.pass))
            .ok_or("static paint material pass out of range")?;
        let job = asset
            .mesh
            .jobs
            .get(paint.geometry_job)
            .ok_or("static paint geometry job out of range")?;
        if job.material != paint.material_slot
            || job.program != Program::Static
            || pass.program != Program::Static
            || asset
                .mesh
                .pass_jobs
                .get(paint.material_slot)
                .and_then(|jobs| jobs.get(paint.pass))
                .copied()
                != Some(paint.geometry_job)
        {
            return Err("static paint geometry mapping mismatch".into());
        }
        if job.vertices.is_empty() || job.indices.is_empty() {
            return Ok(false);
        }
        let geometry_key = (Rc::as_ptr(&asset.mesh) as usize, paint.geometry_job);
        if let Entry::Vacant(entry) = self.geometry.entry(geometry_key) {
            let uploaded = StaticJob::upload(ctx, &asset.mesh, paint.geometry_job)?;
            entry.insert((Rc::clone(&asset.mesh), uploaded));
        }
        let recipe = &pass.shader;
        let key = Rc::as_ptr(recipe) as usize;
        let state = recipe.state.borrow().clone();
        if self
            .materials
            .get(&key)
            .is_none_or(|(_, saved, _)| !saved.same_gpu_program(&state))
        {
            let compiled = Material::compile(ctx, &state, self.depth_bits)?;
            self.compilations += 1;
            if let Some((_, _, previous)) = self
                .materials
                .insert(key, (Rc::clone(recipe), state.clone(), compiled))
            {
                previous.delete(ctx);
            }
        }
        let compiled = &self.materials[&key].2;
        let images = compiled
            .images
            .iter()
            .map(|(_, image)| texture(ctx, recipe, *image))
            .collect::<Result<Vec<_>, _>>()?;
        compiled.draw(
            ctx,
            &self.geometry[&geometry_key].1,
            &images,
            &state,
            environment,
        )?;
        Ok(true)
    }
    pub fn shadow_resource_counts(&self) -> (usize, usize) {
        (self.shadow_geometry.len(), self.shadow_materials.len())
    }
    pub fn draw_shadow(
        &mut self,
        ctx: &mut dyn RenderingBackend,
        asset: &crate::scene::paint::Asset,
        paint: &crate::scene::paint::MeshPaint,
        environment: &Environment,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        use std::{collections::btree_map::Entry, rc::Rc};
        let pass = asset
            .materials
            .get(paint.material_slot)
            .and_then(|m| m.passes.get(paint.pass))
            .ok_or("shadow paint material pass out of range")?;
        let job = asset
            .mesh
            .jobs
            .get(paint.geometry_job)
            .ok_or("shadow paint geometry job out of range")?;
        if job.material != paint.material_slot
            || job.program != Program::Shadow
            || pass.program != Program::Shadow
            || paint.paint.light.is_none()
            || asset
                .mesh
                .pass_jobs
                .get(paint.material_slot)
                .and_then(|jobs| jobs.get(paint.pass))
                .copied()
                != Some(paint.geometry_job)
        {
            return Err("shadow paint geometry mapping mismatch".into());
        }
        if job.vertices.is_empty() {
            return Ok(false);
        }
        let recipe = &pass.shader;
        let state = recipe.state.borrow().clone();
        let light = state.constants(environment).uniforms.vc[4][..3]
            .try_into()
            .unwrap();
        let volume = self.shadow_context.build(&asset.mesh, job, light)?;
        let zfail = paint.paint.zfail;
        if volume.draw_indices(zfail).is_empty() {
            return Ok(false);
        }
        let geometry_key = (Rc::as_ptr(&asset.mesh) as usize, paint.geometry_job);
        if let Entry::Vacant(entry) = self.shadow_geometry.entry(geometry_key) {
            let gpu = ShadowJob::new(ctx, &volume, job.edges.len() * 6 + job.indices.len())?;
            entry.insert((Rc::clone(&asset.mesh), gpu));
        }
        let gpu = &mut self.shadow_geometry.get_mut(&geometry_key).unwrap().1;
        gpu.update(ctx, &volume, zfail)?;
        let key = (Rc::as_ptr(recipe) as usize, zfail);
        if self
            .shadow_materials
            .get(&key)
            .is_none_or(|(_, saved, _)| !saved.same_gpu_program(&state))
        {
            let compiled = Material::compile_shadow(ctx, &state, self.depth_bits, zfail)?;
            self.compilations += 1;
            if let Some((_, _, previous)) = self
                .shadow_materials
                .insert(key, (Rc::clone(recipe), state.clone(), compiled))
            {
                previous.delete(ctx);
            }
        }
        self.shadow_materials[&key]
            .2
            .draw_shadow(ctx, gpu, &state, environment)?;
        Ok(true)
    }
    pub fn delete(self, ctx: &mut dyn RenderingBackend) {
        for (_, (_, job)) in self.geometry {
            job.delete(ctx);
        }
        for (_, (_, job)) in self.shadow_geometry {
            job.delete(ctx);
        }
        for (_, (_, _, material)) in self.shadow_materials {
            material.delete(ctx);
        }
        for (_, (_, _, material)) in self.materials {
            material.delete(ctx);
        }
    }
}

/// Reusable streaming storage for the original 32-byte text/particle vertices.
pub struct EffectJob {
    vertex: BufferId,
    index: BufferId,
    vertex_capacity: usize,
    index_capacity: usize,
    index_count: i32,
}
impl EffectJob {
    pub fn new(
        ctx: &mut dyn RenderingBackend,
        vertices: usize,
        indices: usize,
    ) -> Result<Self, &'static str> {
        if vertices == 0
            || indices == 0
            || vertices > u32::MAX as usize
            || indices > i32::MAX as usize
        {
            return Err("invalid effect stream capacity");
        }
        Ok(Self {
            vertex: ctx.new_buffer(
                BufferType::VertexBuffer,
                BufferUsage::Stream,
                BufferSource::empty::<[f32; 8]>(vertices),
            ),
            index: ctx.new_buffer(
                BufferType::IndexBuffer,
                BufferUsage::Stream,
                BufferSource::empty::<u32>(indices),
            ),
            vertex_capacity: vertices,
            index_capacity: indices,
            index_count: 0,
        })
    }
    pub fn update(
        &mut self,
        ctx: &mut dyn RenderingBackend,
        vertices: &[[f32; 8]],
        indices: &[u32],
    ) -> Result<(), &'static str> {
        if vertices.len() > self.vertex_capacity
            || indices.len() > self.index_capacity
            || !indices.len().is_multiple_of(3)
            || indices
                .iter()
                .any(|&index| index as usize >= vertices.len())
        {
            return Err("invalid effect stream geometry");
        }
        if !indices.is_empty() {
            ctx.buffer_update(self.vertex, BufferSource::slice(vertices));
            ctx.buffer_update(self.index, BufferSource::slice(indices));
        }
        self.index_count = indices.len() as i32;
        Ok(())
    }
    pub fn delete(self, ctx: &mut dyn RenderingBackend) {
        ctx.delete_buffer(self.vertex);
        ctx.delete_buffer(self.index);
    }
}

pub struct EffectView<'a> {
    pub camera: &'a crate::scene::camera::Environment,
    pub dimensions: [i32; 2],
}
/// Submit deferred Print/PartSystem leaves with their original first material
/// pass and reusable stream storage. Render target selection belongs to the caller.
pub struct EffectCache {
    depth_bits: u32,
    compilations: usize,
    stream: Option<EffectJob>,
    materials: std::collections::BTreeMap<usize, (std::rc::Rc<material::Recipe>, State, Material)>,
}
impl EffectCache {
    pub fn new(depth_bits: u32) -> Self {
        Self {
            depth_bits,
            compilations: 0,
            stream: None,
            materials: Default::default(),
        }
    }
    pub fn compilation_count(&self) -> usize {
        self.compilations
    }
    pub fn resource_counts(&self) -> (usize, usize) {
        (usize::from(self.stream.is_some()), self.materials.len())
    }
    pub fn draw(
        &mut self,
        ctx: &mut dyn RenderingBackend,
        content: &crate::scene::effects::Content,
        material: &material::Material,
        view: EffectView<'_>,
        mut texture: impl FnMut(
            &mut dyn RenderingBackend,
            &material::Recipe,
            material::shader::Image,
        ) -> Result<TextureId, Box<dyn std::error::Error>>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        use crate::scene::effects::Content;
        use std::rc::Rc;
        let Some(pass) = material.passes.first() else {
            return Ok(false);
        };
        let particle_vertices;
        let (vertices, indices, environment) = match content {
            Content::Print(draw) => (
                draw.geometry.vertices.as_slice(),
                draw.geometry
                    .indices
                    .iter()
                    .map(|&i| u32::from(i))
                    .collect::<Vec<_>>(),
                draw.environment.material_environment(view.dimensions)?,
            ),
            Content::Particles(quads) => {
                if quads.len() > u32::MAX as usize / 4 {
                    return Err("particle effect exceeds index storage".into());
                }
                particle_vertices = quads
                    .iter()
                    .flat_map(|quad| quad.iter().copied())
                    .collect::<Vec<_>>();
                let indices = (0..quads.len() as u32)
                    .flat_map(|quad| {
                        let base = quad * 4;
                        // Original sQuad macro, unlike Print's explicit triangle order.
                        [base, base + 1, base + 2, base, base + 2, base + 3]
                    })
                    .collect();
                let mut environment = view.camera.material_environment(view.dimensions)?;
                environment.model = crate::mesh::identity();
                (particle_vertices.as_slice(), indices, environment)
            }
        };
        if vertices.is_empty() || indices.is_empty() {
            return Ok(false);
        }
        if self.stream.as_ref().is_none_or(|stream| {
            vertices.len() > stream.vertex_capacity || indices.len() > stream.index_capacity
        }) {
            let stream = EffectJob::new(
                ctx,
                vertices.len().next_power_of_two(),
                indices.len().next_power_of_two(),
            )?;
            if let Some(previous) = self.stream.replace(stream) {
                previous.delete(ctx);
            }
        }
        let stream = self.stream.as_mut().unwrap();
        stream.update(ctx, vertices, &indices)?;
        let recipe = &pass.shader;
        let key = Rc::as_ptr(recipe) as usize;
        let state = recipe.state.borrow().clone();
        if self
            .materials
            .get(&key)
            .is_none_or(|(_, saved, _)| !saved.same_gpu_program(&state))
        {
            let compiled = Material::compile_effect(ctx, &state, self.depth_bits)?;
            self.compilations += 1;
            if let Some((_, _, previous)) = self
                .materials
                .insert(key, (Rc::clone(recipe), state.clone(), compiled))
            {
                previous.delete(ctx);
            }
        }
        let compiled = &self.materials[&key].2;
        let images = compiled
            .images
            .iter()
            .map(|(_, image)| texture(ctx, recipe, *image))
            .collect::<Result<Vec<_>, _>>()?;
        compiled.draw_effect(ctx, stream, &images, &state, &environment)?;
        Ok(true)
    }
    pub fn delete(self, ctx: &mut dyn RenderingBackend) {
        if let Some(stream) = self.stream {
            stream.delete(ctx);
        }
        for (_, (_, _, material)) in self.materials {
            material.delete(ctx);
        }
    }
}

/// D3D9 samples at integer pixel centers; OpenGL samples at half-integers.
/// Keep source projection/constants unchanged on the CPU and translate clip
/// coordinates by half a viewport pixel only at OpenGL submission.
fn raster_uniforms(state: &State, environment: &Environment) -> material::uniforms::Uniforms {
    use miniquad::native::gl::*;
    let mut uniforms = state.constants(environment).uniforms;
    let mut viewport = [0; 4];
    unsafe {
        glGetIntegerv(0x0ba2, viewport.as_mut_ptr());
    }
    if viewport[2] > 0 && viewport[3] > 0 {
        for i in 0..4 {
            uniforms.vc[0][i] += uniforms.vc[3][i] / viewport[2] as f32;
            uniforms.vc[1][i] -= uniforms.vc[3][i] / viewport[3] as f32;
        }
    }
    uniforms
}

/// Original sFVF_STANDARD quad stream used by mesh sprites and thick lines.
pub struct StandardJob(EffectJob);
impl StandardJob {
    pub fn new(ctx: &mut dyn RenderingBackend, quads: usize) -> Result<Self, &'static str> {
        let vertices = quads
            .checked_mul(4)
            .ok_or("standard stream capacity overflow")?;
        let indices = quads
            .checked_mul(6)
            .ok_or("standard stream capacity overflow")?;
        let job = EffectJob::new(ctx, vertices, indices)?;
        let indices: Vec<_> = (0..quads as u32)
            .flat_map(|quad| {
                let base = quad * 4;
                [base, base + 1, base + 2, base, base + 2, base + 3]
            })
            .collect();
        ctx.buffer_update(job.index, BufferSource::slice(&indices));
        Ok(Self(job))
    }
    pub fn update(
        &mut self,
        ctx: &mut dyn RenderingBackend,
        vertices: &[crate::mesh_sprite::Vertex],
    ) -> Result<(), &'static str> {
        let index_count = vertices.len() / 4 * 6;
        if !vertices.len().is_multiple_of(4)
            || vertices.len() > self.0.vertex_capacity
            || index_count > self.0.index_capacity
        {
            return Err("invalid standard quad stream geometry");
        }
        if !vertices.is_empty() {
            ctx.buffer_update(self.0.vertex, BufferSource::slice(vertices));
        }
        self.0.index_count = index_count as i32;
        Ok(())
    }
    pub fn delete(self, ctx: &mut dyn RenderingBackend) {
        self.0.delete(ctx);
    }
}

pub struct StandardCache {
    depth_bits: u32,
    compilations: usize,
    stream: Option<StandardJob>,
    materials: std::collections::BTreeMap<usize, (std::rc::Rc<material::Recipe>, State, Material)>,
}
impl StandardCache {
    pub fn new(depth_bits: u32) -> Self {
        Self {
            depth_bits,
            compilations: 0,
            stream: None,
            materials: Default::default(),
        }
    }
    pub fn compilation_count(&self) -> usize {
        self.compilations
    }
    pub fn resource_counts(&self) -> (usize, usize) {
        (usize::from(self.stream.is_some()), self.materials.len())
    }
    pub fn draw(
        &mut self,
        ctx: &mut dyn RenderingBackend,
        asset: &crate::scene::paint::Asset,
        paint: &crate::scene::paint::MeshPaint,
        environment: &Environment,
        mut texture: impl FnMut(
            &mut dyn RenderingBackend,
            &material::Recipe,
            material::shader::Image,
        ) -> Result<TextureId, Box<dyn std::error::Error>>,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        use std::rc::Rc;
        let pass = asset
            .materials
            .get(paint.material_slot)
            .and_then(|m| m.passes.get(paint.pass))
            .ok_or("standard paint material pass out of range")?;
        let job = asset
            .mesh
            .jobs
            .get(paint.geometry_job)
            .ok_or("standard paint geometry job out of range")?;
        if !matches!(pass.program, Program::Sprites | Program::ThickLines)
            || job.program != pass.program
            || job.material != paint.material_slot
            || asset
                .mesh
                .pass_jobs
                .get(paint.material_slot)
                .and_then(|p| p.get(paint.pass))
                .copied()
                != Some(paint.geometry_job)
        {
            return Err("standard paint geometry mapping mismatch".into());
        }
        let model_view = material::uniforms::mul4(environment.model, environment.world_to_camera);
        let batches = match pass.program {
            Program::Sprites => {
                crate::mesh_sprite::batches(&asset.mesh, job, pass.size, model_view)?
            }
            Program::ThickLines => {
                crate::mesh_line::batches(&asset.mesh, job, pass.size, pass.aspect, model_view)?
            }
            _ => unreachable!(),
        };
        if batches.is_empty() {
            return Ok(0);
        }
        if self.stream.is_none() {
            self.stream = Some(StandardJob::new(ctx, 4096)?);
        }
        let recipe = &pass.shader;
        let key = Rc::as_ptr(recipe) as usize;
        let state = recipe.state.borrow().clone();
        if self
            .materials
            .get(&key)
            .is_none_or(|(_, saved, _)| !saved.same_gpu_program(&state))
        {
            let compiled = Material::compile_standard(ctx, &state, self.depth_bits)?;
            self.compilations += 1;
            if let Some((_, _, previous)) = self
                .materials
                .insert(key, (Rc::clone(recipe), state.clone(), compiled))
            {
                previous.delete(ctx);
            }
        }
        let compiled = &self.materials[&key].2;
        let images = compiled
            .images
            .iter()
            .map(|(_, image)| texture(ctx, recipe, *image))
            .collect::<Result<Vec<_>, _>>()?;
        let stream = self.stream.as_mut().unwrap();
        for vertices in &batches {
            stream.update(ctx, vertices)?;
            compiled.draw_standard(ctx, stream, &images, &state, environment)?;
        }
        Ok(batches.len())
    }
    pub fn delete(self, ctx: &mut dyn RenderingBackend) {
        if let Some(stream) = self.stream {
            stream.delete(ctx);
        }
        for (_, (_, _, material)) in self.materials {
            material.delete(ctx);
        }
    }
}
