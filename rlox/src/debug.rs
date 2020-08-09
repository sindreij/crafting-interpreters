use std::convert::TryInto;

use crate::{
    chunk::{Chunk, OpCode},
    value::Value,
};

pub fn disassemble_chunk(chunk: &Chunk, name: &str) {
    println!("== {} ==", name);

    let mut offset = 0;
    while offset < chunk.code.len() {
        offset = disassemble_instruction(chunk, offset);
    }
}

pub fn disassemble_instruction(chunk: &Chunk, offset: usize) -> usize {
    print!("{:04} ", offset);
    let instruction = chunk.code[offset].try_into();

    match instruction {
        Ok(instruction) => match instruction {
            OpCode::Return => simple_instruction(instruction, offset),
            OpCode::Constant => constant_instruction(instruction, chunk, offset),
        },
        Err(err) => {
            println!("Unknown opcode: {}", err.number);
            offset + 1
        }
    }
}

fn constant_instruction(instruction: OpCode, chunk: &Chunk, offset: usize) -> usize {
    let constant = chunk.code[offset + 1];
    println!(
        "{:16} {:04} '{}'",
        instruction,
        constant,
        chunk.constant(constant)
    );

    offset + 2
}

fn simple_instruction(instruction: OpCode, offset: usize) -> usize {
    println!("{}", instruction);
    offset + 1
}
