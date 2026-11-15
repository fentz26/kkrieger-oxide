//! Original scene initialization. Mesh/effect handles await CPU preparation;
//! dynamic Exec handlers are deliberately separate from this cached scene tree.
use crate::{kx::Document, mesh_graph};
use std::{cell::RefCell, fmt, rc::Rc};

pub mod delay;
pub mod event;
pub mod exec;
pub mod instance;
pub mod menu;
pub mod trigger;

pub type Handle = Rc<RefCell<Scene>>;
#[derive(Clone, Debug)]
pub struct Scene {
    pub children: Vec<Handle>,
    pub mesh: Option<usize>,
    pub effect: Option<usize>,
    pub srt: [f32; 9],
    pub count: i32,
    pub is_sector: bool,
}
impl Default for Scene {
    fn default() -> Self {
        Self {
            children: Vec::new(),
            mesh: None,
            effect: None,
            srt: [1., 1., 1., 0., 0., 0., 0., 0., 0.],
            count: 0,
            is_sector: false,
        }
    }
}
#[derive(Debug)]
pub struct Error {
    pub operator: usize,
    pub reason: &'static str,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "scene op {}: {}", self.operator, self.reason)
    }
}
impl std::error::Error for Error {}

fn new_scene() -> Handle {
    Rc::new(RefCell::new(Scene::default()))
}
fn make(doc: &Document<'_>, index: usize, cache: &[Option<Handle>]) -> Result<Handle, Error> {
    if mesh_graph::implemented(doc.classes[doc.operators[index].class].id) {
        let scene = new_scene();
        scene.borrow_mut().mesh = Some(index);
        Ok(scene)
    } else if matches!(
        doc.classes[doc.operators[index].class].id,
        0x61 | 0x63 | 0x64
    ) {
        let scene = new_scene();
        scene.borrow_mut().effect = Some(index);
        Ok(scene)
    } else {
        cache[index].clone().ok_or(Error {
            operator: index,
            reason: "input has no scene initialization handler",
        })
    }
}

/// Constructs source Init_Scene_* caches, preserving pointer sharing. This does
/// not claim the cached tree is an executed frame (monsters, portals and walking
/// issue their jobs from Exec handlers rather than their Init trees).
pub fn initialize_many(doc: &Document<'_>, roots: &[usize]) -> Result<Vec<Handle>, Error> {
    Ok(initialize_with_sectors(doc, roots)?.roots)
}

pub struct Initialized {
    pub roots: Vec<Handle>,
    pub sectors: Vec<(usize, Handle)>,
}

/// Retain sector identities from the same initialization pass as the roots.
pub fn initialize_with_sectors(doc: &Document<'_>, roots: &[usize]) -> Result<Initialized, Error> {
    let mut needed = vec![false; doc.operators.len()];
    let mut pending = roots.to_vec();
    while let Some(i) = pending.pop() {
        let op = doc.operators.get(i).ok_or(Error {
            operator: i,
            reason: "root out of range",
        })?;
        if needed[i] {
            continue;
        }
        needed[i] = true;
        let id = doc.classes[op.class].id;
        if mesh_graph::implemented(id) || matches!(id, 0x61 | 0x63 | 0x64) {
            continue;
        }
        // Monster Init returns an empty scene after releasing its input caches.
        if id == 0x11 {
            continue;
        }
        if id == 0x01 {
            pending.extend(op.links.first().copied().flatten());
        } else {
            pending.extend(op.inputs.iter().copied());
        }
    }
    let mut cache: Vec<Option<Handle>> = vec![None; needed.len()];
    for (i, op) in doc.operators.iter().enumerate().filter(|(i, _)| needed[*i]) {
        let id = doc.classes[op.class].id;
        if mesh_graph::implemented(id) || matches!(id, 0x61 | 0x63 | 0x64) {
            continue;
        }
        let input = |n: usize| -> Result<Handle, Error> {
            make(
                doc,
                *op.inputs.get(n).ok_or(Error {
                    operator: i,
                    reason: "missing scene input",
                })?,
                &cache,
            )
        };
        let srt = || -> Result<[f32; 9], Error> {
            op.parameters
                .get(..9)
                .map(|p| std::array::from_fn(|n| f32::from_bits(p[n])))
                .ok_or(Error {
                    operator: i,
                    reason: "truncated scene transform",
                })
        };
        let result = match id {
            0x01 => make(
                doc,
                op.links.first().copied().flatten().ok_or(Error {
                    operator: i,
                    reason: "missing load link",
                })?,
                &cache,
            )?,
            0x02..=0x0c | 0x0e | 0x15 => input(0)?,
            0x11 | 0x13 | 0x14 | 0xc4 => new_scene(),
            0x10 | 0x12 => {
                if op.inputs.is_empty() {
                    new_scene()
                } else {
                    input(0)?
                }
            }
            0xc0 => {
                if op.inputs.is_empty() {
                    new_scene()
                } else {
                    let scene = input(0)?;
                    scene.borrow_mut().srt = srt()?;
                    scene
                }
            }
            0xc1 | 0xc7 => {
                let scene = new_scene();
                for n in 0..op.inputs.len() {
                    scene.borrow_mut().children.push(input(n)?);
                }
                scene
            }
            0xc2 | 0xc3 => {
                let scene = new_scene();
                scene.borrow_mut().children.push(input(0)?);
                scene.borrow_mut().srt = srt()?;
                if id == 0xc2 {
                    scene.borrow_mut().count = *op.parameters.get(9).ok_or(Error {
                        operator: i,
                        reason: "missing scene instance count",
                    })? as i32;
                }
                scene
            }
            0xc5 | 0xc8 | 0xc9 | 0xce => {
                let scene = new_scene();
                if !op.inputs.is_empty() {
                    scene.borrow_mut().children.push(input(0)?);
                }
                scene
            }
            0xc6 => {
                let scene = new_scene();
                scene.borrow_mut().srt = srt()?;
                scene
            }
            0xcb => {
                let scene = new_scene();
                if !op.inputs.is_empty() {
                    scene.borrow_mut().children.push(input(0)?);
                }
                scene.borrow_mut().is_sector = true;
                scene
            }
            0xcd => {
                if op.inputs.is_empty() {
                    new_scene()
                } else {
                    input(0)?
                }
            }
            _ => {
                return Err(Error {
                    operator: i,
                    reason: "scene initialization handler has not been ported",
                });
            }
        };
        cache[i] = Some(result);
    }
    let roots = roots
        .iter()
        .map(|&i| make(doc, i, &cache))
        .collect::<Result<_, _>>()?;
    let sectors = cache
        .into_iter()
        .enumerate()
        .filter_map(|(i, scene)| {
            if doc.classes[doc.operators[i].class].id == 0xcb {
                scene.map(|scene| (i, scene))
            } else {
                None
            }
        })
        .collect();
    Ok(Initialized { roots, sectors })
}

/// Cache identity for scene-valued aliases, mirroring the Init handlers. Mesh
/// and effect wrappers create new scenes; aliases retain their source identity.
pub fn cache_scene_identity(doc: &Document<'_>, operator: usize) -> Result<Option<usize>, Error> {
    #[derive(Clone, Copy)]
    enum Object {
        Scene(usize),
        Mesh,
        Effect,
        Other,
    }
    fn resolve(doc: &Document<'_>, index: usize, depth: usize) -> Result<Object, Error> {
        let fail = |reason| Error {
            operator: index,
            reason,
        };
        if depth > doc.operators.len() {
            return Err(fail("scene cache alias cycle"));
        }
        let op = doc
            .operators
            .get(index)
            .ok_or_else(|| fail("scene cache index out of range"))?;
        let id = doc.classes[op.class].id;
        let input = |i| resolve(doc, i, depth + 1);
        Ok(match id {
            0x01 => {
                if let Some(i) = op.links.first().copied().flatten() {
                    input(i)?
                } else {
                    Object::Other
                }
            }
            0x02..=0x0c | 0x0e | 0x15 | 0xcd => {
                if let Some(&i) = op.inputs.first() {
                    input(i)?
                } else if id == 0xcd {
                    Object::Scene(index)
                } else {
                    Object::Other
                }
            }
            0x10 | 0x12 => {
                if let Some(&i) = op.inputs.first() {
                    input(i)?
                } else {
                    Object::Scene(index)
                }
            }
            0xc0 => {
                if let Some(&i) = op.inputs.first() {
                    match input(i)? {
                        Object::Scene(identity) => Object::Scene(identity),
                        Object::Mesh | Object::Effect => Object::Scene(index),
                        Object::Other => Object::Other,
                    }
                } else {
                    Object::Scene(index)
                }
            }
            0xc2 | 0xc3 => {
                if let Some(&i) = op.inputs.first() {
                    match input(i)? {
                        Object::Other => Object::Other,
                        _ => Object::Scene(index),
                    }
                } else {
                    Object::Other
                }
            }
            0x11 | 0x13 | 0x14 | 0xc1 | 0xc4..=0xc9 | 0xcb | 0xce => Object::Scene(index),
            0x61 | 0x63 | 0x64 => Object::Effect,
            _ => {
                if mesh_graph::implemented(id) {
                    Object::Mesh
                } else {
                    Object::Other
                }
            }
        })
    }
    Ok(match resolve(doc, operator, 0)? {
        Object::Scene(index) => Some(index),
        _ => None,
    })
}
