use std::convert::TryFrom;

use crate::{
    chunk::{Chunk, OpCode},
    debug::disassemble_instruction,
    value::Value,
};

static DEBUG_TRACE_EXECUTION: bool = true;

pub struct VM<'a> {
    chunk: &'a Chunk,
    ip: usize,
}

#[derive(Debug)]
pub enum InterpretError {
    CompileError,
    RuntimeError,
}

impl std::fmt::Display for InterpretError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for InterpretError {}

impl<'a> VM<'a> {
    pub fn new<'chunk>(chunk: &'chunk Chunk) -> VM<'chunk> {
        VM { chunk, ip: 0 }
    }

    #[inline]
    fn read_byte(&mut self) -> u8 {
        let res = self.chunk.code()[self.ip];
        self.ip += 1;
        res
    }

    #[inline]
    fn read_constant(&mut self) -> &Value {
        self.chunk.constant(self.read_byte())
    }

    pub fn run(&mut self) -> Result<(), InterpretError> {
        loop {
            if DEBUG_TRACE_EXECUTION {
                disassemble_instruction(self.chunk, self.ip);
            }

            let instruction = OpCode::try_from(self.read_byte());

            match instruction {
                Ok(instruction) => match instruction {
                    OpCode::Return => return Ok(()),
                    OpCode::Constant => {
                        let constant = self.read_constant();
                        println!("{}", constant);
                    }
                },
                Err(err) => {
                    panic!("Error reading instruction: {}", err);
                }
            }
        }
    }
}
