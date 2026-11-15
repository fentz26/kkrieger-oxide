//! Original SetIf, State and Menu switch/key semantics.
use crate::game::input::{BACKSPACE, DOWN, ENTER, ESCAPE, LEFT, MOUSE_LEFT, RIGHT, UP};
pub struct Comparison {
    pub variable: Option<usize>,
    pub value: bool,
    pub execute: bool,
}
pub fn set_if(
    switches: &mut [u8; 256],
    switch: usize,
    value: i32,
    flags: u32,
    output: usize,
) -> Result<Comparison, &'static str> {
    let mut compare = i32::from(
        *switches
            .get(switch)
            .ok_or("comparison switch out of range")?,
    )
    .wrapping_sub(value);
    if flags & 0x20 != 0 {
        compare = i32::from(compare < 0);
    }
    let mut compare = compare == 0;
    if flags & 0x10 != 0 {
        compare = !compare;
    }
    let kind = flags & 15;
    match kind {
        0..=3 => Ok(Comparison {
            variable: Some(kind as usize),
            value: compare,
            execute: true,
        }),
        4..=6 => {
            if kind == 4 || compare {
                let out = switches
                    .get_mut(output)
                    .ok_or("comparison output switch out of range")?;
                *out = if kind == 4 {
                    u8::from(compare)
                } else {
                    (6 - kind) as u8
                };
            }
            Ok(Comparison {
                variable: None,
                value: compare,
                execute: true,
            })
        }
        _ => Ok(Comparison {
            variable: None,
            value: compare,
            execute: false,
        }),
    }
}
pub fn state(
    switches: &mut [u8; 256],
    key: &mut u32,
    switch: usize,
    value: i32,
    condition: i32,
    condition_switch: usize,
    condition_value: i32,
) -> Result<(), &'static str> {
    let ok = match condition {
        0 => true,
        1 => *key == 32,
        2 => *key == ENTER || *key == MOUSE_LEFT,
        3 => *key == BACKSPACE || *key == ESCAPE,
        4 => *key == UP,
        5 => *key == DOWN,
        6 => *key == LEFT,
        7 => *key == RIGHT,
        _ => false,
    };
    if ok
        && i32::from(
            *switches
                .get(condition_switch)
                .ok_or("state condition switch out of range")?,
        ) == condition_value
        && switch != 0
    {
        *switches
            .get_mut(switch)
            .ok_or("state output switch out of range")? = value as u8;
        if condition != 0 {
            *key = 0;
        }
    }
    Ok(())
}
pub fn menu(
    switches: &mut [u8; 256],
    key: &mut u32,
    switch: usize,
    max: i32,
    flags: u32,
) -> Result<(), &'static str> {
    let direction = if flags & 1 != 0 {
        match *key {
            UP => -1,
            DOWN => 1,
            _ => 0,
        }
    } else {
        match *key {
            LEFT => -1,
            RIGHT => 1,
            _ => 0,
        }
    };
    if direction != 0 {
        let slot = switches.get_mut(switch).ok_or("menu switch out of range")?;
        let value = i32::from(*slot) + direction;
        let high = max.wrapping_sub(1);
        *slot = if value >= high {
            high
        } else if value <= 0 {
            0
        } else {
            value
        } as u8;
        *key = 0;
    }
    Ok(())
}
