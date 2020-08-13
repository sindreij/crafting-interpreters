use std::convert::TryFrom;

use crate::{
    chunk::{Chunk, OpCode},
    compiler::compile,
    debug::disassemble_instruction,
    object::ObjectHeap,
    value::Value,
};

const STACK_MAX: usize = 256;

pub struct VM {
    chunk: Chunk,
    ip: usize,
    stack: [Value; STACK_MAX],
    stack_top: usize,
    heap: ObjectHeap,
}

#[derive(Debug)]
pub enum InterpretError {
    CompileError,
    RuntimeError(RuntimeError),
}

impl std::fmt::Display for InterpretError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InterpretError::CompileError => write!(f, "Compile Error"),
            InterpretError::RuntimeError(inner) => write!(f, "Runtime Error: {}", inner),
        }
    }
}

impl std::error::Error for InterpretError {}

#[derive(Debug)]
pub struct RuntimeError {
    line: usize,
    message: String,
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.message)?;
        writeln!(f, "[line {}] in script", self.line)
    }
}

impl std::error::Error for RuntimeError {}

macro_rules! runtime_error {
    ($vm:expr, $msg:literal $(,)?) => {{
        let instruction = $vm.ip - 1;
        let line = $vm.chunk.line(instruction);
        let message = $msg.to_string();
        return Err(RuntimeError { line, message });
    }};
    ($vm:expr, $fmt:expr, $($arg:tt)*) => {{
        let instruction = $vm.ip - 1;
        let line = $vm.chunk.line(instruction);
        let message = format!($fmt, $($arg)*);
        return Err(RuntimeError { line, message });
    }};
}

macro_rules! binary_op {
    ($vm: expr, $valueType:expr, $op:tt) => {
        {
            use Value::*;
            let b = $vm.pop();
            let a = $vm.pop();
            match (a, b) {
                (Number(a), Number(b)) => {
                    $vm.push($valueType(a $op b));
                },
                _ => runtime_error!($vm, "Operands must be numbers."),
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
            heap: ObjectHeap::new(),
        }
    }

    fn push(&mut self, value: Value) {
        self.stack[self.stack_top] = value;
        self.stack_top += 1;
    }

    fn pop(&mut self) -> Value {
        self.stack_top -= 1;
        self.stack[self.stack_top]
    }

    fn peek(&self, distance: usize) -> Value {
        self.stack[self.stack_top - 1 - distance]
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
        let chunk = compile(source, &mut self.heap).map_err(|()| InterpretError::CompileError)?;

        self.chunk = chunk;
        self.ip = 0;
        self.run().map_err(InterpretError::RuntimeError)
    }

    pub fn run(&mut self) -> Result<(), RuntimeError> {
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
                        self.push(constant);
                    }
                    OpCode::Negate => match self.pop() {
                        Value::Number(value) => self.push(Value::Number(-value)),
                        operand => {
                            runtime_error!(self, "Operand ({}) must be a number", operand);
                        }
                    },
                    OpCode::Add => binary_op!(self, Value::Number, +),
                    OpCode::Subtract => binary_op!(self, Value::Number, -),
                    OpCode::Multiply => binary_op!(self, Value::Number, *),
                    OpCode::Divide => binary_op!(self, Value::Number, /),
                    OpCode::Nil => self.push(Value::Nil),
                    OpCode::True => self.push(Value::Bool(true)),
                    OpCode::False => self.push(Value::Bool(false)),
                    OpCode::Not => {
                        let value = Value::Bool(self.pop().is_falsey());
                        self.push(value);
                    }
                    OpCode::Equal => {
                        let b = self.pop();
                        let a = self.pop();

                        self.push(Value::Bool(a == b));
                    }
                    OpCode::Greater => binary_op!(self, Value::Bool, >),
                    OpCode::Less => binary_op!(self, Value::Bool, <),
                },
                Err(err) => {
                    panic!("Error reading instruction: {}", err);
                }
            }
        }
    }
}
