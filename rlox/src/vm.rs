use std::convert::TryFrom;

use crate::{
    chunk::{Chunk, OpCode},
    debug::disassemble_instruction,
    value::Value,
};

const DEBUG_TRACE_EXECUTION: bool = false;
const STACK_MAX: usize = 256;

pub struct VM<'a> {
    chunk: &'a Chunk,
    ip: usize,
    stack: [Value; STACK_MAX],
    stack_top: usize,
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
        VM {
            chunk,
            ip: 0,
            stack: [Value::Nil; STACK_MAX],
            stack_top: 0,
        }
    }

    fn push(&mut self, value: &Value) {
        self.stack[self.stack_top] = *value;
        self.stack_top += 1;
    }

    fn pop(&mut self) -> Value {
        self.stack_top -= 1;
        self.stack[self.stack_top]
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
                print!("          ");
                for i in 0..self.stack_top {
                    print!("[ {} ]", self.stack[i]);
                }
                println!();
                disassemble_instruction(self.chunk, self.ip);
            }

            let instruction = OpCode::try_from(self.read_byte());

            match instruction {
                Ok(instruction) => match instruction {
                    OpCode::Return => {
                        println!("{}", self.pop());
                        return Ok(());
                    }
                    OpCode::Constant => {
                        let constant = *self.read_constant();
                        self.push(&constant);
                    }
                    OpCode::Negate => match self.pop() {
                        Value::Nil => todo!(),
                        Value::Number(value) => self.push(&Value::Number(-value)),
                    },
                },
                Err(err) => {
                    panic!("Error reading instruction: {}", err);
                }
            }
        }
    }
}
