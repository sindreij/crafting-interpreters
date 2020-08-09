use chunk::{Chunk, OpCode};
use debug::disassemble_chunk;
use value::Value;

mod chunk;
mod debug;
mod value;

fn main() {
    let mut chunk = Chunk::new();

    let constant = chunk.add_constant(Value::Number(1.2));
    chunk.write(OpCode::Constant as u8);
    chunk.write(constant as u8);

    chunk.write(OpCode::Return as u8);

    disassemble_chunk(&chunk, "test chunk");
}
