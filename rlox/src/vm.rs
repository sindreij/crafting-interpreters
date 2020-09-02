use std::{collections::HashMap, convert::TryFrom};

use crate::{
    chunk::OpCode,
    compiler::compile,
    debug::disassemble_instruction,
    object::{ObjFunction, ObjHeap, ObjKind, ObjPointer},
    value::Value,
};

const FRAMES_MAX: usize = 64;
const STACK_MAX: usize = FRAMES_MAX * 0xff;

pub struct VM {
    frames: Vec<CallFrame>,
    stack: [Value; STACK_MAX],
    stack_top: usize,
    heap: ObjHeap,
    globals: HashMap<ObjPointer, Value>,
}

pub struct CallFrame {
    function: ObjPointer,
    ip: usize,
    // clox calls this `slots`, but we cannot have another pointer to
    // the stack without using unsafe
    // fp = frame pointer
    fp: usize,
}

impl CallFrame {
    fn function<'a>(&self, heap: &'a ObjHeap) -> &'a ObjFunction {
        self.function.borrow(heap).as_function()
    }
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
    message: String,
    call_stack: Vec<(usize, String)>,
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.message)?;

        for (line, name) in &self.call_stack {
            writeln!(f, "[line {} in {}]", line, name)?;
        }

        Ok(())
    }
}

impl std::error::Error for RuntimeError {}

macro_rules! runtime_error {
    ($vm:expr, $msg:literal $(,)?) => {{
        let call_stack = $vm.generate_call_stack();
        let frame = $vm.frames.last().unwrap();
        let instruction = frame.ip - 1;
        let message = $msg.to_string();


        return Err(RuntimeError { message, call_stack });
    }};
    ($vm:expr, $fmt:expr, $($arg:tt)*) => {{
        let call_stack = $vm.generate_call_stack();
        let frame = $vm.frames.last().unwrap();
        let instruction = frame.ip - 1;
        let message = format!($fmt, $($arg)*);
        return Err(RuntimeError { message, call_stack });
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

macro_rules! frame {
    ($vm: expr) => {
        $vm.frames.last_mut().unwrap()
    };
}

impl VM {
    pub fn new() -> VM {
        VM {
            stack: [Value::Nil; STACK_MAX],
            stack_top: 0,
            frames: Vec::with_capacity(FRAMES_MAX),
            heap: ObjHeap::new(),
            globals: HashMap::new(),
        }
    }

    pub fn generate_call_stack(&mut self) -> Vec<(usize, String)> {
        self.frames
            .iter()
            .rev()
            .map(|frame| {
                let function = frame.function(&self.heap);
                (
                    function.chunk.line(frame.ip - 1),
                    function
                        .name
                        .as_ref()
                        .map(|name| format!("{}()", name))
                        .unwrap_or_else(|| "script".to_owned()),
                )
            })
            .collect()
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

    fn call_value(&mut self, callee: Value, arg_count: usize) -> Result<(), RuntimeError> {
        match callee {
            Value::Obj(callee_ptr) => match &callee_ptr.borrow(&mut self.heap).kind {
                ObjKind::Function(function) => {
                    let arity = function.arity;
                    self.call(callee_ptr, arg_count, arity)?;
                }
                _ => runtime_error!(self, "Can only call functions and classes"),
            },
            _ => runtime_error!(self, "Can only call functions and classes"),
        }

        Ok(())
    }

    fn call(
        &mut self,
        function: ObjPointer,
        arg_count: usize,
        arity: usize,
    ) -> Result<(), RuntimeError> {
        if arg_count != arity {
            runtime_error!(self, "Expected {} arguments, but got {}", arity, arg_count);
        }

        if self.frames.len() == FRAMES_MAX {
            runtime_error!(self, "Stack overflow!");
        }

        self.frames.push(CallFrame {
            function,
            ip: 0,
            fp: self.stack_top - arg_count - 1,
        });

        Ok(())
    }

    #[inline]
    fn read_byte(&mut self) -> u8 {
        // let res = self.chunk.code[self.ip];
        let mut frame = frame!(self);
        let res = frame.function(&mut self.heap).chunk.code[frame.ip];
        frame.ip += 1;
        res
    }

    #[inline]
    fn read_short(&mut self) -> u16 {
        let frame = frame!(self);
        let function = frame.function(&mut self.heap);
        frame.ip += 2;
        (function.chunk.code[frame.ip - 2] as u16) << 8 | (function.chunk.code[frame.ip - 1] as u16)
    }

    #[inline]
    fn read_constant(&mut self) -> &Value {
        let constant_id = self.read_byte();
        let frame = frame!(self);
        let function = frame.function(&mut self.heap);
        function.chunk.constant(constant_id)
    }

    #[inline]
    fn read_string(&mut self) -> ObjPointer {
        self.read_constant().as_obj_ptr()
    }

    pub fn interpret(&mut self, source: &str) -> Result<(), InterpretError> {
        let function =
            compile(source, &mut self.heap).map_err(|()| InterpretError::CompileError)?;

        let function = self.heap.allocate_obj(ObjKind::Function(function));
        let function = Value::Obj(function);

        self.push(function);
        self.call_value(function, 0)
            .map_err(InterpretError::RuntimeError)?;

        self.run().map_err(InterpretError::RuntimeError)
    }

    pub fn run(&mut self) -> Result<(), RuntimeError> {
        loop {
            if std::env::var("TRACE_EXECUTION").ok().as_deref() == Some("true") {
                print!("          ");
                for i in 0..self.stack_top {
                    print!("[ {} ]", self.stack[i].to_string(&self.heap));
                }
                println!();
                let chunk = &frame!(self).function(&mut self.heap).chunk.clone();
                disassemble_instruction(chunk, frame!(self).ip, &self.heap);
            }

            let instruction = OpCode::try_from(self.read_byte());

            match instruction {
                Ok(instruction) => match instruction {
                    OpCode::Return => {
                        return Ok(());
                    }
                    OpCode::Constant => {
                        let constant = *self.read_constant();
                        self.push(constant);
                    }
                    OpCode::Negate => match self.pop() {
                        Value::Number(value) => self.push(Value::Number(-value)),
                        operand => {
                            runtime_error!(self, "Operand ({:?}) must be a number", operand);
                        }
                    },
                    OpCode::Add => match (self.pop(), self.pop()) {
                        (Value::Number(b), Value::Number(a)) => self.push(Value::Number(a + b)),
                        (Value::Obj(b), Value::Obj(a)) => {
                            let new_obj =
                                match (&a.borrow(&self.heap).kind, &b.borrow(&self.heap).kind) {
                                    (ObjKind::String(a), ObjKind::String(b)) => {
                                        let mut new_string =
                                            String::with_capacity(a.len() + b.len());
                                        new_string.push_str(a);
                                        new_string.push_str(b);
                                        Value::Obj(self.heap.take_string(new_string))
                                    }
                                    _ => runtime_error!(
                                        self,
                                        "Operands must be two numbers or two strings"
                                    ),
                                };
                            self.push(new_obj);
                        }
                        _ => runtime_error!(self, "Operands must be two numbers or two strings"),
                    },
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

                        self.push(Value::Bool(a.eq(&b)));
                    }
                    OpCode::Greater => binary_op!(self, Value::Bool, >),
                    OpCode::Less => binary_op!(self, Value::Bool, <),
                    OpCode::Print => {
                        println!("{}", self.pop().to_string(&self.heap));
                    }
                    OpCode::Pop => {
                        self.pop();
                    }
                    OpCode::GetGlobal => {
                        let name = self.read_string();
                        let value = match self.globals.get(&name) {
                            Some(value) => *value,
                            None => runtime_error!(
                                self,
                                "Undefined variable '{}'",
                                name.to_string(&self.heap)
                            ),
                        };
                        self.push(value);
                    }
                    OpCode::DefineGlobal => {
                        let name = self.read_string();
                        self.globals.insert(name, self.peek(0));
                        self.pop();
                    }
                    OpCode::SetGlobal => {
                        let name = self.read_string();
                        if !self.globals.contains_key(&name) {
                            runtime_error!(
                                self,
                                "Undefined variable '{}'",
                                name.to_string(&self.heap)
                            );
                        }
                        self.globals.insert(name, self.peek(0));
                        // No POP since a `set` is a expression and should return the value
                    }
                    OpCode::GetLocal => {
                        let slot = self.read_byte() as usize;
                        // self.push(self.stack[slot as usize]);
                        let value = self.stack[frame!(self).fp + slot];
                        self.push(value);
                    }
                    OpCode::SetLocal => {
                        let slot = self.read_byte() as usize;
                        self.stack[frame!(self).fp + slot] = self.peek(0);
                    }
                    OpCode::JumpIfFalse => {
                        let offset = self.read_short();
                        if self.peek(0).is_falsey() {
                            frame!(self).ip += offset as usize;
                        }
                    }
                    OpCode::Jump => {
                        let offset = self.read_short();
                        frame!(self).ip += offset as usize;
                    }
                    OpCode::Loop => {
                        let offset = self.read_short();
                        frame!(self).ip -= offset as usize;
                    }
                    OpCode::Call => {
                        let arg_count = self.read_byte() as usize;
                        self.call_value(self.peek(arg_count), arg_count)?;
                    }
                },
                Err(err) => {
                    panic!("Error reading instruction: {}", err);
                }
            }
        }
    }
}
