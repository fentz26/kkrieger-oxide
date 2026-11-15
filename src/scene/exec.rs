//! Scene Exec handlers emit the original engine jobs. Unported gameplay is an
//! error, so an incomplete frame can never masquerade as a faithful game frame.
use super::{
    Error, delay,
    instance::{Memory, Scope},
};
use crate::{
    animation::Vm,
    kx::Document,
    mesh::{Matrix, identity, srt_matrix},
    mesh_graph,
    visibility::{self, Portal, Rect},
};

#[derive(Clone, Debug)]
pub enum Job {
    Paint {
        operator: usize,
        matrix: Matrix,
        time: f32,
        pass_adjust: i32,
    },
    Light {
        operator: usize,
        position: [f32; 4],
        flags: u32,
        color: u32,
        amplify: f32,
        range: f32,
        clip: Rect,
    },
    Portal {
        operator: usize,
        portal: Portal,
    },
    Sector {
        operator: usize,
        input: usize,
        matrix: Matrix,
    },
}

pub struct Execution {
    pub vm: Vm,
    pub camera: Matrix,
    pub pass_adjust: i32,
    pub jobs: Vec<Job>,
    pub switches: [u8; 256],
    pub current_time: i32,
    pub last_time: i32,
    instances: Memory,
    parameters: Vec<Vec<u32>>,
    current_clip: Rect,
    visibility_history: Vec<Rect>,
}
impl Execution {
    pub fn new(doc: &Document<'_>) -> Self {
        Self {
            vm: Vm::new(),
            camera: identity(),
            pass_adjust: 0,
            jobs: Vec::new(),
            switches: [0; 256],
            current_time: 0,
            last_time: 0,
            instances: Memory::default(),
            parameters: doc.operators.iter().map(|o| o.parameters.clone()).collect(),
            current_clip: Rect::UNIT,
            visibility_history: Vec::new(),
        }
    }
    pub fn frame(&mut self, doc: &Document<'_>, root: usize) -> Result<(), Error> {
        self.jobs.clear();
        self.current_clip = Rect::UNIT;
        self.visibility_history.clear();
        self.instances.enter(Scope::Default);
        let result = self.visit(doc, root, identity());
        if result.is_err() {
            self.jobs.clear();
        }
        result
    }
    /// Execute deferred sector contents in original newest-first order. Sector
    /// visibility is the enabled source branch; instance/game handlers still
    /// report errors until their per-instance runtime has been ported.
    pub fn process_sectors(
        &mut self,
        doc: &Document<'_>,
        observer: Option<usize>,
        projection: Matrix,
    ) -> Result<(), Error> {
        use std::collections::BTreeMap;
        let sectors: Vec<_> = self
            .jobs
            .iter()
            .filter_map(|job| match job {
                Job::Sector {
                    operator,
                    input,
                    matrix,
                } => Some((*operator, *input, *matrix)),
                _ => None,
            })
            .collect();
        let ids: BTreeMap<_, _> = sectors
            .iter()
            .enumerate()
            .map(|(i, (id, _, _))| (*id, i))
            .collect();
        let result = (|| {
            if ids.len() != sectors.len() {
                return Err(Error {
                    operator: 0,
                    reason: "sector scene registered more than once",
                });
            }
            let fail = |reason| Error {
                operator: observer.unwrap_or(0),
                reason,
            };
            let index = |id: usize| {
                ids.get(&id)
                    .copied()
                    .ok_or_else(|| fail("portal scene has no registered sector"))
            };
            let mut portals = Vec::new();
            if observer.is_some() {
                for job in &self.jobs {
                    if let Job::Portal { portal, .. } = job {
                        let mut portal = portal.clone();
                        portal.sectors = [index(portal.sectors[0])?, index(portal.sectors[1])?];
                        portal.between = portal.between.map(index).transpose()?;
                        portals.push(portal);
                    }
                }
            }
            let observer = observer.map(index).transpose()?;
            let visible = visibility::compute_with_history(
                sectors.len(),
                &portals,
                observer,
                projection,
                &self.visibility_history,
            )
            .map_err(fail)?;
            self.visibility_history = visible.iter().map(|s| s.bounds).collect();
            for (i, &(_, input, matrix)) in sectors.iter().enumerate().rev() {
                if visible[i].execute {
                    self.current_clip = visible[i].bounds;
                    self.visit_scene(doc, input, matrix)?;
                }
            }
            Ok(())
        })();
        self.current_clip = Rect::UNIT;
        if result.is_err() {
            self.jobs.clear();
            self.visibility_history.clear();
        }
        result
    }
    fn visit_scene(
        &mut self,
        doc: &Document<'_>,
        root: usize,
        matrix: Matrix,
    ) -> Result<(), Error> {
        let saved = self.instances.enter(Scope::Scene(root));
        let result = self.visit(doc, root, matrix);
        self.instances.restore(saved);
        result
    }
    fn inputs(&mut self, doc: &Document<'_>, root: usize, matrix: Matrix) -> Result<(), Error> {
        for &input in &doc.operators[root].inputs {
            let class = doc.classes[doc.operators[input].class].id;
            if mesh_graph::implemented(class) || matches!(class, 0x61 | 0x63 | 0x64) {
                self.jobs.push(Job::Paint {
                    operator: input,
                    matrix,
                    time: self.vm.variables[0][0],
                    pass_adjust: self.pass_adjust,
                });
            } else {
                self.visit(doc, input, matrix)?;
            }
        }
        Ok(())
    }
    fn visit(&mut self, doc: &Document<'_>, root: usize, parent: Matrix) -> Result<(), Error> {
        let fail = |reason| Error {
            operator: root,
            reason,
        };
        let op = doc
            .operators
            .get(root)
            .ok_or_else(|| fail("execution root out of range"))?;
        let id = doc.classes[op.class].id;
        let p = self
            .parameters
            .get_mut(root)
            .ok_or_else(|| fail("execution document mismatch"))?;
        let eval = self
            .vm
            .execute(op, p, &doc.splines, None)
            .map_err(|_| fail("scene animation failed"))?;
        let p = p.clone();
        let f = |i| {
            p.get(i)
                .copied()
                .map(f32::from_bits)
                .ok_or_else(|| fail("truncated scene parameters"))
        };
        let srt = || -> Result<Matrix, Error> {
            let p = p
                .get(..9)
                .ok_or_else(|| fail("truncated scene transform"))?;
            Ok(srt_matrix(&std::array::from_fn(|i| f32::from_bits(p[i]))))
        };
        let result = (|| match id {
            0x08 => {
                let flags = *p.first().ok_or_else(|| fail("missing delay flags"))?;
                let switch = *p.get(1).ok_or_else(|| fail("missing delay switch"))? as usize;
                let switch = if flags & 0xc0 == 0 {
                    *self
                        .switches
                        .get(switch)
                        .ok_or_else(|| fail("delay switch out of range"))?
                } else {
                    0
                };
                let parameters = delay::Parameters {
                    flags,
                    threshold: f(2)?,
                    hysteresis: f(3)?,
                    durations: [f(4)?, f(5)?, f(6)?],
                };
                self.instances.delay(root)?.advance(
                    parameters,
                    switch,
                    &mut self.vm.variables,
                    self.current_time,
                    self.last_time,
                );
                if let Some(&input) = op.inputs.first() {
                    self.visit(doc, input, parent)?;
                }
                Ok(())
            }
            0x09 => {
                let switch = *p.first().ok_or_else(|| fail("missing If switch"))? as usize;
                let value = *p.get(1).ok_or_else(|| fail("missing If value"))? as i32;
                let switch = *self
                    .switches
                    .get(switch)
                    .ok_or_else(|| fail("If switch out of range"))?;
                let index = usize::from(switch as i32 != value);
                if let Some(&input) = op.inputs.get(index) {
                    self.visit_scene(doc, input, parent)?;
                }
                Ok(())
            }
            0x02 | 0x03 | 0xc1 => self.inputs(doc, root, parent),
            0xc0 | 0xc3 => self.inputs(doc, root, multiply(srt()?, parent)),
            0xc2 => {
                let count = *p.get(9).ok_or_else(|| fail("missing instance count"))? as i32;
                let step = srt()?;
                let saved = self.vm.variables[3];
                let mut matrix = parent;
                let result = (|| {
                    for i in 0..count {
                        self.vm.variables[3] = [i as f32; 4];
                        self.inputs(doc, root, matrix)?;
                        matrix = multiply(step, matrix);
                    }
                    Ok(())
                })();
                self.vm.variables[3] = saved;
                result
            }
            0xc4 => {
                let local = srt_matrix(&[1., 1., 1., f(0)?, f(1)?, f(2)?, f(3)?, f(4)?, f(5)?]);
                self.jobs.push(Job::Light {
                    operator: root,
                    position: multiply(local, parent)[3],
                    flags: *p.get(6).ok_or_else(|| fail("missing light flags"))?,
                    color: *p.get(8).ok_or_else(|| fail("missing light color"))?,
                    amplify: f(9)?,
                    range: f(10)?,
                    clip: self.current_clip,
                });
                Ok(())
            }
            0xc6 => {
                self.camera = multiply(srt()?, parent);
                Ok(())
            }
            0xcb => {
                if let Some(&input) = op.inputs.first() {
                    self.jobs.push(Job::Sector {
                        operator: root,
                        input,
                        matrix: parent,
                    });
                }
                Ok(())
            }
            0xcd => {
                let mut links = [None; 3];
                for (n, link) in links.iter_mut().enumerate() {
                    if let Some(i) = op.links.get(n).copied().flatten() {
                        *link = super::cache_scene_identity(doc, i)?;
                    }
                }
                if let [Some(a), Some(b), between] = links {
                    let bounds = [[f(0)?, f(2)?, f(4)?], [f(1)?, f(3)?, f(5)?]];
                    let cost = if f(7)? >= 0.02 {
                        *p.get(6).ok_or_else(|| fail("missing portal cost"))? as i32
                    } else {
                        256
                    };
                    self.jobs.push(Job::Portal {
                        operator: root,
                        portal: Portal::new([a, b], between, bounds, parent, cost),
                    });
                }
                if let Some(&input) = op.inputs.first() {
                    self.visit(doc, input, parent)?;
                }
                Ok(())
            }
            _ => Err(fail("scene execution handler has not been ported")),
        })();
        self.vm
            .restore(eval.saved_variables)
            .map_err(|_| fail("scene variable restore failed"))?;
        result
    }
}

/// Original affine MulA, preserving source evaluation order.
pub fn multiply(a: Matrix, b: Matrix) -> Matrix {
    let mut out = [[0.; 4]; 4];
    for n in 0..4 {
        for axis in 0..3 {
            out[n][axis] = b[0][axis] * a[n][0] + b[1][axis] * a[n][1] + b[2][axis] * a[n][2];
        }
    }
    for axis in 0..3 {
        out[3][axis] += b[3][axis];
    }
    out[3][3] = 1.;
    out
}
