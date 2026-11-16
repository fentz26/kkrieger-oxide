//! Original four-component animation VM and spline evaluator in safe Rust.
//! Farbrausch BSD-2-Clause: third_party/farbrausch-LICENSE.txt.
use crate::{
    kx::{Error, Operator, Spline},
    random,
};

type V4 = [f32; 4];
type Result<T> = std::result::Result<T, Error>;
const STACK: usize = 256;

/// Original KOp::UpdateVar range: explicit LOADVAR instructions and the ranges
/// of called inputs/links. KA_MATRIX does not extend this source-level range.
pub fn effect_variable_range(
    doc: &crate::kx::Document<'_>,
    root: usize,
) -> Result<Option<std::ops::Range<usize>>> {
    let mut pending = vec![root];
    let mut visited = vec![false; doc.operators.len()];
    let mut range: Option<std::ops::Range<usize>> = None;
    while let Some(index) = pending.pop() {
        let op = doc
            .operators
            .get(index)
            .ok_or(err(index, "effect dependency out of range"))?;
        if visited[index] {
            continue;
        }
        visited[index] = true;
        let mut pc = 0;
        loop {
            let command = take(op.animation, &mut pc, 1)?[0];
            if command == 1 {
                break;
            }
            let count = match command {
                0 | 7..=21 | 26..=28 => 0,
                2..=6 | 0x80..=0xbf | 0xd0..=0xff => 1,
                22 => 16,
                23..=24 => 4,
                25 => 2,
                29 => 8,
                _ => return Err(err(pc, "unknown effect animation opcode")),
            };
            let payload = take(op.animation, &mut pc, count)?;
            if command == 2 {
                let variable = payload[0] as usize;
                if variable >= 32 {
                    return Err(err(pc, "effect variable out of range"));
                }
                if let Some(r) = &mut range {
                    r.start = r.start.min(variable);
                    r.end = r.end.max(variable + 1);
                } else {
                    range = Some(variable..variable + 1);
                }
            }
        }
        pending.extend(op.inputs.iter().copied());
        if doc.classes[op.class].convention & 0x1000_0000 == 0 {
            pending.extend(op.links.iter().flatten().copied());
        }
    }
    Ok(range)
}

impl Spline {
    pub fn evaluate(&self, time: f32) -> V4 {
        let mut value = [0.; 4];
        if time.is_nan() {
            return value;
        }
        for (dst, keys) in value.iter_mut().zip(&self.channels) {
            if keys.is_empty() {
                continue;
            }
            if time <= keys[0].time {
                *dst = keys[0].value;
                continue;
            }
            if time >= keys[keys.len() - 1].time {
                *dst = keys[keys.len() - 1].value;
                continue;
            }
            for k in 0..keys.len() - 1 {
                let a = &keys[k];
                let b = &keys[k + 1];
                if b.time <= time {
                    continue;
                }
                let dif = b.value - a.value;
                let tdif = b.time - a.time;
                let t = (time - a.time) / tdif;
                *dst = match self.interpolation {
                    0 => {
                        let d0 = if k == 0 {
                            dif
                        } else {
                            tdif * (b.value - keys[k - 1].value) / (b.time - keys[k - 1].time)
                        };
                        let d1 = if k == keys.len() - 2 {
                            dif
                        } else {
                            tdif * (keys[k + 2].value - a.value) / (keys[k + 2].time - a.time)
                        };
                        (((d0 + d1 - 2. * dif) * t + 3. * dif - 2. * d0 - d1) * t + d0) * t
                            + a.value
                    }
                    1 => t * dif + a.value,
                    2 => a.value,
                    _ => 0.,
                };
                break;
            }
        }
        value
    }
}

pub struct Vm {
    pub variables: [V4; 32],
    saved: [(usize, V4); STACK],
    save_len: usize,
    permutation: [u8; 256],
}
#[derive(Debug)]
pub struct Evaluation {
    pub saved_variables: usize,
    pub changed: bool,
    pub read_variables: u32,
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}
impl Vm {
    pub fn new() -> Self {
        let mut vm = Self {
            variables: [[0.; 4]; 32],
            saved: [(0, [0.; 4]); STACK],
            save_len: 0,
            permutation: random::permutation(),
        };
        for i in 0..4 {
            vm.variables[28 + i][i] = 1.;
        }
        vm
    }
    pub fn begin_frame(&mut self, beat: i32) {
        self.variables.fill([0.; 4]);
        self.variables[0] = [beat as f32 / 65536.; 4];
        self.variables[4] = [1., 1., 1., 0.];
        self.saved.fill((0, [0.; 4]));
        self.save_len = 0;
    }
    pub fn restore(&mut self, count: usize) -> Result<()> {
        if count > self.save_len {
            return Err(err(0, "variable restore underflow"));
        }
        for _ in 0..count {
            self.save_len -= 1;
            let (i, value) = self.saved[self.save_len];
            self.variables[i] = value;
        }
        Ok(())
    }
    pub fn execute(
        &mut self,
        op: &Operator<'_>,
        parameters: &mut [u32],
        splines: &[Spline],
        event_spline: Option<&Spline>,
    ) -> Result<Evaluation> {
        let code = op.animation;
        let mut pc = 0;
        let mut stack = [[0.; 4]; STACK];
        let mut sp = 0;
        let before = self.save_len;
        let mut changed = false;
        let mut read_variables = 0;
        loop {
            let command = take(code, &mut pc, 1)?[0];
            match command {
                0 => {}
                1 => {
                    if sp != 0 {
                        return Err(err(pc, "animation leaves operands on stack"));
                    }
                    if pc != code.len() {
                        return Err(err(pc, "bytes after animation END"));
                    }
                    return Ok(Evaluation {
                        saved_variables: self.save_len - before,
                        changed,
                        read_variables,
                    });
                }
                2 => {
                    let i = take(code, &mut pc, 1)?[0] as usize;
                    let v = *self
                        .variables
                        .get(i)
                        .ok_or(err(pc, "variable out of range"))?;
                    read_variables |= 1 << i;
                    push(&mut stack, &mut sp, v, pc)?;
                }
                3..=6 => {
                    let i = take(code, &mut pc, 1)?[0] as usize;
                    let count = (command - 2) as usize;
                    let words = op
                        .parameters
                        .get(i..i + count)
                        .ok_or(err(pc, "load parameter out of range"))?;
                    let mut value = [0.; 4];
                    if count == 1 {
                        value.fill(f32::from_bits(words[0]));
                    } else {
                        for (v, &word) in value.iter_mut().zip(words) {
                            *v = f32::from_bits(word);
                        }
                    }
                    push(&mut stack, &mut sp, value, pc)?;
                }
                7..=10 => {
                    let a = top(&mut stack, sp, pc)?;
                    *a = [a[(command - 7) as usize]; 4];
                }
                11..=15 => {
                    let b = pop(&stack, &mut sp, pc)?;
                    let a = top(&mut stack, sp, pc)?;
                    for (a, b) in a.iter_mut().zip(b) {
                        *a = match command {
                            11 => *a + b,
                            12 => *a - b,
                            13 => *a * b,
                            14 => *a / b,
                            _ => *a % b,
                        };
                    }
                }
                16..=21 => {
                    let a = top(&mut stack, sp, pc)?;
                    match command {
                        16 => {
                            for v in a {
                                *v = 1. - *v;
                            }
                        }
                        17 => {
                            for v in a {
                                *v = -*v;
                            }
                        }
                        18 => *a = [(a[0] * std::f32::consts::TAU).sin(); 4],
                        19 => *a = [(a[0] * std::f32::consts::TAU).cos(); 4],
                        20 => *a = [if a[0].rem_euclid(1.) < 0.5 { 1. } else { 0. }; 4],
                        _ => *a = [a[0].rem_euclid(1.); 4],
                    }
                }
                22 => {
                    let mut value = [0.; 4];
                    for v in &mut value {
                        *v = read_float(code, &mut pc)?;
                    }
                    push(&mut stack, &mut sp, value, pc)?;
                }
                23 => {
                    let bytes = take(code, &mut pc, 4)?;
                    push(
                        &mut stack,
                        &mut sp,
                        std::array::from_fn(|i| bytes[i] as f32 / 255.),
                        pc,
                    )?;
                }
                24 => {
                    let v = read_float(code, &mut pc)?;
                    push(&mut stack, &mut sp, [v; 4], pc)?;
                }
                25 => {
                    let i =
                        u16::from_le_bytes(take(code, &mut pc, 2)?.try_into().unwrap()) as usize;
                    let spline = splines
                        .get(i)
                        .ok_or(err(pc, "animation spline out of range"))?;
                    let value = top(&mut stack, sp, pc)?;
                    *value = spline.evaluate(value[0]);
                }
                26 => {
                    let value = top(&mut stack, sp, pc)?;
                    *value = event_spline
                        .map(|s| s.evaluate(value[0]))
                        .unwrap_or([0.; 4]);
                }
                27 => {
                    let a = top(&mut stack, sp, pc)?;
                    let original = *a;
                    *a = std::array::from_fn(|r| {
                        self.variables[28][r] * original[0]
                            + self.variables[29][r] * original[1]
                            + self.variables[30][r] * original[2]
                            + self.variables[31][r] * original[3]
                    });
                    read_variables |= 0xf0000000;
                }
                28 => {
                    let a = top(&mut stack, sp, pc)?;
                    let i = (a[0].rem_euclid(1.) * 65536.) as usize;
                    let noise = self.permutation[(i >> 8) & 255] as u16
                        | ((self.permutation[i & 255] as u16) << 8);
                    *a = [noise as f32 / 65535.; 4];
                }
                29 => {
                    let ei = read_float(code, &mut pc)?.min(0.5);
                    let mut eo = 1. - read_float(code, &mut pc)?.min(0.5);
                    let mut ei = ei;
                    let d = eo - ei;
                    if d < 0.1 {
                        eo += (0.1 - d) / 2.;
                        ei -= (0.1 - d) / 2.;
                    }
                    let s = 1. / (eo - ei);
                    for v in top(&mut stack, sp, pc)? {
                        let f = *v;
                        let f = if f < 0. {
                            -ei * s / 2.
                        } else if f < ei {
                            -ei * s / 2. + f * f * s / (2. * ei)
                        } else if f < eo {
                            (f - ei) * s
                        } else if f < 1. {
                            1. + (1. - eo) * s / 2. - (1. - f) * (1. - f) * s / (2. * (1. - eo))
                        } else {
                            1. + (1. - eo) * s / 2.
                        };
                        *v = (f + ei * s / 2.) / (1. + ei * s / 2. + (1. - eo) * s / 2.);
                    }
                }
                0x80..=0xbf | 0xd0..=0xff => {
                    let i = take(code, &mut pc, 1)?[0] as usize;
                    let value = pop(&stack, &mut sp, pc)?;
                    let kind = command & 0xf0;
                    let mask = command & 15;
                    if kind == 0x80 {
                        let var = self
                            .variables
                            .get_mut(i)
                            .ok_or(err(pc, "store variable out of range"))?;
                        if self.save_len == STACK {
                            return Err(err(pc, "variable save stack overflow"));
                        }
                        self.saved[self.save_len] = (i, *var);
                        self.save_len += 1;
                        for c in 0..4 {
                            if mask & (1 << c) != 0 {
                                var[c] = value[c];
                            }
                        }
                    } else {
                        for (c, &v) in value.iter().enumerate() {
                            if mask & (1 << c) == 0 {
                                continue;
                            }
                            if kind == 0xb0 || kind == 0xf0 {
                                let word = parameters
                                    .get_mut(i)
                                    .ok_or(err(pc, "byte parameter store out of range"))?;
                                let b = (v * 255.).clamp(0., 255.) as u8;
                                let old = (*word >> (c * 8)) & 255;
                                if kind == 0xf0 {
                                    changed |= old != b as u32;
                                }
                                *word = (*word & !(255 << (c * 8))) | ((b as u32) << (c * 8));
                            } else {
                                let word = parameters
                                    .get_mut(i + c)
                                    .ok_or(err(pc, "parameter store out of range"))?;
                                if kind == 0x90 || kind == 0xd0 {
                                    if kind == 0xd0 {
                                        changed |= f32::from_bits(*word) != v;
                                    }
                                    *word = v.to_bits();
                                } else {
                                    if kind == 0xe0 {
                                        changed |= (*word as i32) as f32 != v;
                                    }
                                    *word = (v as i32) as u32;
                                }
                            }
                        }
                    }
                }
                _ => return Err(err(pc, "unknown animation instruction")),
            }
        }
    }
}
fn err(offset: usize, reason: &'static str) -> Error {
    Error { offset, reason }
}
fn take<'a>(code: &'a [u8], pc: &mut usize, count: usize) -> Result<&'a [u8]> {
    let end = pc
        .checked_add(count)
        .ok_or(err(*pc, "bytecode offset overflow"))?;
    let data = code.get(*pc..end).ok_or(err(*pc, "truncated bytecode"))?;
    *pc = end;
    Ok(data)
}
fn read_float(code: &[u8], pc: &mut usize) -> Result<f32> {
    Ok(f32::from_le_bytes(take(code, pc, 4)?.try_into().unwrap()))
}
fn push(stack: &mut [V4; STACK], sp: &mut usize, v: V4, pc: usize) -> Result<()> {
    let dst = stack
        .get_mut(*sp)
        .ok_or(err(pc, "operand stack overflow"))?;
    *dst = v;
    *sp += 1;
    Ok(())
}
fn top(stack: &mut [V4; STACK], sp: usize, pc: usize) -> Result<&mut V4> {
    if sp == 0 {
        return Err(err(pc, "operand stack underflow"));
    }
    Ok(&mut stack[sp - 1])
}
fn pop(stack: &[V4; STACK], sp: &mut usize, pc: usize) -> Result<V4> {
    if *sp == 0 {
        return Err(err(pc, "operand stack underflow"));
    }
    *sp -= 1;
    Ok(stack[*sp])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kx::{Document, Key};
    const DATA: &[u8] = include_bytes!("../assets/kkrieger3383.kx");
    fn execute(code: &[u8], words: Vec<u32>, vm: &mut Vm) -> Result<(Vec<u32>, Evaluation)> {
        let mut d = Document::parse(DATA).unwrap();
        let op = &mut d.operators[0];
        op.animation = code;
        op.parameters = words.clone();
        let mut animated = words;
        let result = vm.execute(op, &mut animated, &d.splines, None)?;
        Ok((animated, result))
    }
    fn constant(value: V4) -> Vec<u8> {
        let mut code = vec![22];
        for v in value {
            code.extend(v.to_le_bytes());
        }
        code
    }
    #[test]
    fn scoped_variables_restore_and_stores_respect_masks() {
        let mut vm = Vm::new();
        vm.variables[5] = [10., 20., 30., 40.];
        let mut code = constant([1., 2., 3., 4.]);
        code.extend([0x83, 5, 2, 5, 0x9f, 0, 1]);
        let (out, r) = execute(&code, vec![0; 4], &mut vm).unwrap();
        assert_eq!(
            out.into_iter().map(f32::from_bits).collect::<Vec<_>>(),
            [1., 2., 30., 40.]
        );
        assert_eq!(r.saved_variables, 1);
        assert!(!r.changed);
        assert_eq!(r.read_variables, 1 << 5);
        vm.restore(r.saved_variables).unwrap();
        assert_eq!(vm.variables[5], [10., 20., 30., 40.]);
        assert!(vm.restore(1).is_err());
    }
    #[test]
    fn byte_integer_and_changing_float_stores_match_original_semantics() {
        let mut vm = Vm::new();
        let mut code = constant([-1., 0.5, 2., 1.]);
        code.extend([0xbf, 0, 1]);
        let (out, _) = execute(&code, vec![0], &mut vm).unwrap();
        assert_eq!(out, [0xffff7f00]);
        let mut code = constant([1.2, -2.9, 3., 4.]);
        code.extend([0xe3, 0, 1]);
        let (out, r) = execute(&code, vec![1, (-2_i32) as u32], &mut vm).unwrap();
        assert_eq!(out, [1, (-2_i32) as u32]);
        assert!(r.changed);
        let mut code = constant([-0., 0., 0., 0.]);
        code.extend([0xd1, 0, 1]);
        let (out, r) = execute(&code, vec![0], &mut vm).unwrap();
        assert_eq!(out, [(-0_f32).to_bits()]);
        assert!(!r.changed);
    }
    #[test]
    fn spline_modes_and_negative_cycle_math_match_reference() {
        for (mode, expected) in [(0, 0.5), (1, 0.5), (2, 0.)] {
            let s = Spline {
                interpolation: mode,
                channels: vec![vec![
                    Key {
                        time: 0.,
                        value: 0.,
                    },
                    Key {
                        time: 1.,
                        value: 1.,
                    },
                ]],
            };
            assert_eq!(s.evaluate(0.5), [expected, 0., 0., 0.]);
            assert_eq!(s.evaluate(-1.), [0.; 4]);
            assert_eq!(s.evaluate(2.), [1., 0., 0., 0.]);
        }
        let mut code = constant([-0.25, 1., 2., 3.]);
        code.extend([21, 0x9f, 0, 1]);
        let (out, _) = execute(&code, vec![0; 4], &mut Vm::new()).unwrap();
        assert_eq!(out, [0.75_f32.to_bits(); 4]);
    }
    #[test]
    fn malformed_programs_return_errors_instead_of_panicking() {
        for code in [
            &[11, 1][..],
            &[24, 0][..],
            &[2, 255, 1][..],
            &[0x91, 0, 1][..],
            &[22, 1][..],
            &[1, 1][..],
            &[0x40, 1][..],
        ] {
            assert!(execute(code, vec![0; 4], &mut Vm::new()).is_err());
        }
        let mut code = vec![];
        for _ in 0..257 {
            code.push(24);
            code.extend(0_f32.to_le_bytes());
        }
        code.push(1);
        assert!(execute(&code, vec![0; 4], &mut Vm::new()).is_err());
    }
    #[test]
    fn every_published_animation_executes_at_multiple_times() {
        let d = Document::parse(DATA).unwrap();
        let mut count = 0;
        for time in [0., 0.25, 1., 17.375, 1024.] {
            for (i, op) in d
                .operators
                .iter()
                .enumerate()
                .filter(|(_, op)| op.animation != [1])
            {
                let mut vm = Vm::new();
                vm.variables[0] = [time; 4];
                vm.variables[1] = [0.75; 4];
                let mut p = op.parameters.clone();
                let result = vm
                    .execute(op, &mut p, &d.splines, None)
                    .unwrap_or_else(|e| panic!("operator {i}: {e}"));
                vm.restore(result.saved_variables).unwrap();
                count += 1;
            }
        }
        assert_eq!(count, 113 * 5);
    }
}
