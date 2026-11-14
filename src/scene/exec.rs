//! Scene Exec handlers emit the original engine jobs. Unported gameplay is an
//! error, so an incomplete frame can never masquerade as a faithful game frame.
use super::Error;
use crate::{
    animation::Vm,
    kx::Document,
    mesh::{Matrix, identity, srt_matrix},
    mesh_graph,
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
    parameters: Vec<Vec<u32>>,
}
impl Execution {
    pub fn new(doc: &Document<'_>) -> Self {
        Self {
            vm: Vm::new(),
            camera: identity(),
            pass_adjust: 0,
            jobs: Vec::new(),
            parameters: doc.operators.iter().map(|o| o.parameters.clone()).collect(),
        }
    }
    pub fn frame(&mut self, doc: &Document<'_>, root: usize) -> Result<(), Error> {
        self.jobs.clear();
        let result = self.visit(doc, root, identity());
        if result.is_err() {
            self.jobs.clear();
        }
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
