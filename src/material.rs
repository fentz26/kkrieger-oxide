//! Original Material11 pass scheduling. Shader compilation is a separate stage.
use crate::kx::Document;
use std::{cell::RefCell, fmt, rc::Rc};
pub mod lookup;
mod pipeline;
pub mod shader;
mod state;
pub mod uniforms;
pub use state::State;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Program {
    Static,
    Shadow,
    Sprites,
    ThickLines,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Usage {
    Base,
    PreLight,
    Ambient,
    Light,
    Shadow,
    PostLight,
    PostLight2,
    Other,
    Explicit(u32),
}

#[derive(Clone, Debug)]
pub struct Pass {
    /// Shader identity remains shared when Material_Add copies a pass.
    pub shader: Rc<Recipe>,
    pub usage: Usage,
    pub program: Program,
    pub render_pass: u32,
    pub size: f32,
    pub aspect: f32,
}

#[derive(Debug)]
pub struct Recipe {
    pub operator: usize,
    pub phase: Usage,
    pub parameters: [u32; 64],
    pub textures: [Option<usize>; 8],
    pub state: RefCell<State>,
}

#[derive(Clone, Debug, Default)]
pub struct Material {
    pub passes: Vec<Pass>,
    /// Material_Add intentionally does not propagate the insert pointer.
    pub material11_insert: bool,
}

#[derive(Debug)]
pub struct Error {
    pub operator: usize,
    pub reason: &'static str,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "material op {}: {}", self.operator, self.reason)
    }
}
impl std::error::Error for Error {}

pub fn generate_many(doc: &Document<'_>, roots: &[usize]) -> Result<Vec<Rc<Material>>, Error> {
    let fail = |operator, reason| Error { operator, reason };
    let mut needed = vec![false; doc.operators.len()];
    let mut pending = roots.to_vec();
    while let Some(i) = pending.pop() {
        let op = doc
            .operators
            .get(i)
            .ok_or_else(|| fail(i, "root out of range"))?;
        if needed[i] {
            continue;
        }
        if !matches!(doc.classes[op.class].id, 0xd0 | 0xd1) {
            return Err(fail(i, "expected a material operator"));
        }
        needed[i] = true;
        pending.extend(op.inputs.iter().copied());
    }
    let mut cache: Vec<Option<Rc<Material>>> = vec![None; needed.len()];
    for (i, op) in doc.operators.iter().enumerate().filter(|(i, _)| needed[*i]) {
        let mut material = Material::default();
        for &input in &op.inputs {
            let source = cache
                .get(input)
                .and_then(Option::as_ref)
                .ok_or_else(|| fail(i, "material input is not available"))?;
            material.passes.extend(source.passes.iter().cloned());
        }
        if doc.classes[op.class].id == 0xd0 {
            let parameters: [u32; 64] = op
                .parameters
                .get(..64)
                .and_then(|p| p.try_into().ok())
                .ok_or_else(|| fail(i, "truncated material parameters"))?;
            let textures = std::array::from_fn(|n| {
                op.links
                    .get(n)
                    .copied()
                    .flatten()
                    .filter(|&link| doc.classes[doc.operators[link].class].id < 0x80)
            });
            let flags = parameters[48];
            let mut append = |phase, program, size, aspect| {
                material.passes.push(Pass {
                    shader: Rc::new(Recipe {
                        operator: i,
                        phase,
                        parameters,
                        textures,
                        state: RefCell::new(State::build(&parameters, textures, phase)),
                    }),
                    usage: phase,
                    program,
                    render_pass: parameters[49],
                    size,
                    aspect,
                });
            };
            if flags & 0x1e == 0 {
                let phase = match (flags >> 20) & 15 {
                    0 => Usage::Other,
                    n => Usage::from_raw(n - 1),
                };
                let finalizer = (flags >> 16) & 3;
                let program = match finalizer {
                    1 => Program::Sprites,
                    2 => Program::ThickLines,
                    _ => Program::Static,
                };
                let (size, aspect) = if finalizer != 0 {
                    (
                        f32::from_bits(parameters[60]),
                        f32::from_bits(parameters[62]),
                    )
                } else {
                    (1.0, 1.0)
                };
                append(phase, program, size, aspect);
            } else {
                append(Usage::Base, Program::Static, 1.0, 1.0);
                for (mask, phase, program) in [
                    (2, Usage::Light, Program::Static),
                    (4, Usage::Shadow, Program::Shadow),
                    (8, Usage::PostLight, Program::Static),
                    (16, Usage::PostLight2, Program::Static),
                ] {
                    if flags & mask != 0 {
                        append(phase, program, 1.0, 1.0);
                    }
                }
                material.material11_insert = op.inputs.is_empty();
            }
        }
        cache[i] = Some(Rc::new(material));
    }
    roots
        .iter()
        .map(|&i| cache[i].clone().ok_or_else(|| fail(i, "missing output")))
        .collect()
}

impl Usage {
    pub fn as_raw(self) -> u32 {
        match self {
            Self::Base => 0,
            Self::PreLight => 1,
            Self::Ambient => 2,
            Self::Shadow => 3,
            Self::Light => 4,
            Self::PostLight => 5,
            Self::PostLight2 => 6,
            Self::Other => 7,
            Self::Explicit(value) => value,
        }
    }
    pub fn from_raw(value: u32) -> Self {
        match value {
            0 => Self::Base,
            1 => Self::PreLight,
            2 => Self::Ambient,
            3 => Self::Shadow,
            4 => Self::Light,
            5 => Self::PostLight,
            6 => Self::PostLight2,
            7 => Self::Other,
            n => Self::Explicit(n),
        }
    }
}
impl Material {
    pub fn execute(&mut self, parameters: &[u32; 64]) {
        for pass in &mut self.passes {
            pass.shader
                .state
                .borrow_mut()
                .execute(parameters, pass.usage);
            if pass.usage == Usage::Other && parameters[48] & 0x30000 != 0 {
                pass.size = f32::from_bits(parameters[60]);
                pass.aspect = f32::from_bits(parameters[62]);
            }
        }
    }
}
