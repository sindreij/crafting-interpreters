use chunk::{Chunk, OpCode};
use debug::disassemble_chunk;
use value::Value;

mod chunk;
mod debug;
mod value;

fn main() {
    let mut chunk = Chunk::new();

    let constant = chunk.add_constant(Value::Number(1.2));
    chunk.write_op(OpCode::Constant);
    chunk.write(constant);

    chunk.write_op(OpCode::Return);

    disassemble_chunk(&chunk, "test chunk");
}
