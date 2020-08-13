use std::convert::TryFrom;

use crate::{
    chunk::{Chunk, OpCode},
    compiler::compile,
    debug::disassemble_instruction,
    value::Value,
};

const STACK_MAX: usize = 256;

pub struct VM {
    chunk: Chunk,
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

macro_rules! binary_op {
    ($vm: expr, $op:tt) => {
        {
            use Value::*;
            let b = $vm.pop();
            let a = $vm.pop();
            match (a, b) {
                (Number(a), Number(b)) => {
                    $vm.push(&Value::Number(a $op b));
                },
                _ => todo!(),
            }
        }
    };
}

impl VM {
    pub fn new() -> VM {
        VM {
            chunk: Chunk::new(),
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
        let constant_id = self.read_byte();
        self.chunk.constant(constant_id)
    }

    pub fn interpret(&mut self, source: &str) -> Result<(), InterpretError> {
        let chunk = compile(source).map_err(|()| InterpretError::CompileError)?;

        self.chunk = chunk;
        self.ip = 0;
        self.run()
    }

    pub fn run(&mut self) -> Result<(), InterpretError> {
        loop {
            if std::env::var("TRACE_EXECUTION").ok().as_deref() == Some("true") {
                print!("          ");
                for i in 0..self.stack_top {
                    print!("[ {} ]", self.stack[i]);
                }
                println!();
                disassemble_instruction(&self.chunk, self.ip);
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
                    OpCode::Add => binary_op!(self, +),
                    OpCode::Subtract => binary_op!(self, -),
                    OpCode::Multiply => binary_op!(self, *),
                    OpCode::Divide => binary_op!(self, /),
                },
                Err(err) => {
                    panic!("Error reading instruction: {}", err);
                }
            }
        }
    }
}
