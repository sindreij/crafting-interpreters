use num_enum::{IntoPrimitive, TryFromPrimitive};

use crate::value::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum OpCode {
    Return,
    // Constant,
}

pub struct Chunk {
    pub code: Vec<u8>,
    pub constants: Vec<Value>,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            constants: Vec::new(),
        }
    }

    pub fn write_op(&mut self, op: OpCode) {
        self.write(op as u8);
    }

    pub fn write(&mut self, byte: u8) {
        self.code.push(byte);
    }

    pub fn add_constant(&mut self, value: Value) -> usize {
        self.constants.push(value);
        self.constants.len() - 1
    }
}
