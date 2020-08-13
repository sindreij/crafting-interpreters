use std::convert::TryInto;

use crate::{
    chunk::{Chunk, OpCode},
    object::ObjHeap,
};

pub fn disassemble_chunk(chunk: &Chunk, name: &str, heap: &ObjHeap) {
    println!("== {} ==", name);

    let mut offset = 0;
    while offset < chunk.code().len() {
        offset = disassemble_instruction(chunk, offset, heap);
    }
}

pub fn disassemble_instruction(chunk: &Chunk, offset: usize, heap: &ObjHeap) -> usize {
    print!("{:04} ", offset);
    if offset > 0 && chunk.line(offset) == chunk.line(offset - 1) {
        print!("   | ");
    } else {
        print!("{:4} ", chunk.line(offset));
    }
    let instruction = chunk.code()[offset].try_into();

    match instruction {
        Ok(instruction) => match instruction {
            OpCode::Return => simple_instruction(instruction, offset),
            OpCode::Constant => constant_instruction(instruction, chunk, offset, heap),
            OpCode::Negate => simple_instruction(instruction, offset),
            OpCode::Add => simple_instruction(instruction, offset),
            OpCode::Subtract => simple_instruction(instruction, offset),
            OpCode::Multiply => simple_instruction(instruction, offset),
            OpCode::Divide => simple_instruction(instruction, offset),
            OpCode::Nil => simple_instruction(instruction, offset),
            OpCode::True => simple_instruction(instruction, offset),
            OpCode::False => simple_instruction(instruction, offset),
            OpCode::Not => simple_instruction(instruction, offset),
            OpCode::Equal => simple_instruction(instruction, offset),
            OpCode::Greater => simple_instruction(instruction, offset),
            OpCode::Less => simple_instruction(instruction, offset),
            OpCode::Print => simple_instruction(instruction, offset),
            OpCode::Pop => simple_instruction(instruction, offset),
        },
        Err(err) => {
            println!("Unknown opcode: {}", err.number);
            offset + 1
        }
    }
}

fn constant_instruction(
    instruction: OpCode,
    chunk: &Chunk,
    offset: usize,
    heap: &ObjHeap,
) -> usize {
    let constant = chunk.code()[offset + 1];
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
