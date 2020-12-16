#![cfg(feature = "print-code")]

use std::convert::TryInto;

use crate::{
    chunk::{Chunk, OpCode},
    object::ObjHeap,
};

pub fn disassemble_chunk(chunk: &Chunk, name: &str, heap: &ObjHeap) {
    println!("== {} ==", name);

    let mut offset = 0;
    while offset < chunk.code.len() {
        offset = disassemble_instruction(chunk, offset, heap);
    }
}

pub fn disassemble_instruction(chunk: &Chunk, offset: usize, heap: &ObjHeap) -> usize {
    use OpCode::*;
    print!("{:04} ", offset);
    if offset > 0 && chunk.line(offset) == chunk.line(offset - 1) {
        print!("   | ");
    } else {
        print!("{:4} ", chunk.line(offset));
    }
    let instruction = chunk.code[offset].try_into();

    match instruction {
        Ok(instruction) => match instruction {
            Constant | DefineGlobal | GetGlobal | SetGlobal => {
                constant_instruction(instruction, chunk, offset, heap)
            }
            Negate | Return | Add | Subtract | Multiply | Divide | Nil | True | False | Not
            | Equal | Greater | Less | Print | Pop => simple_instruction(instruction, offset),
            GetLocal | SetLocal | Call => byte_instruction(instruction, chunk, offset),
            Jump | JumpIfFalse => jump_instruction(instruction, 1, chunk, offset),
            Loop => jump_instruction(instruction, -1, chunk, offset),
            Closure => {
                let constant = chunk.code[offset + 1];
                println!(
                    "{:16} {:4} {}",
                    instruction,
                    constant,
                    chunk.constant(constant).to_string(heap)
                );

                offset + 2
            }
        },
        Err(err) => {
            println!("Unknown opcode: {}", err.number);
            offset + 1
        }
    }
}

fn byte_instruction(instruction: OpCode, chunk: &Chunk, offset: usize) -> usize {
    let slot = chunk.code[offset + 1];
    println!("{:16} {:4}", instruction, slot);

    offset + 2
}

fn jump_instruction(instruction: OpCode, sign: i32, chunk: &Chunk, offset: usize) -> usize {
    let jump = (chunk.code[offset + 1] as u16) << 8 | chunk.code[offset + 2] as u16;
    println!(
        "{:16} {:4} -> {}",
        instruction,
        offset,
        offset as i32 + 3 + sign * jump as i32
    );

    offset + 3
}

fn constant_instruction(
    instruction: OpCode,
    chunk: &Chunk,
    offset: usize,
    heap: &ObjHeap,
) -> usize {
    let constant = chunk.code[offset + 1];
    println!(
        "{:16} {:4} '{}'",
        instruction,
        constant,
        chunk.constant(constant).to_string(heap)
    );

    offset + 2
}

fn simple_instruction(instruction: OpCode, offset: usize) -> usize {
    println!("{}", instruction);
    offset + 1
}
