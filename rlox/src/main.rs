use chunk::{Chunk, OpCode};
use debug::disassemble_chunk;
use value::Value;
use vm::VM;

mod chunk;
mod debug;
mod value;
mod vm;

fn main() {
    let mut chunk = Chunk::new();

    let constant = chunk.add_constant(Value::Number(1.2));
    chunk.write_op(OpCode::Constant, 123);
    chunk.write(constant, 123);

    chunk.write_op(OpCode::Return, 123);

    // disassemble_chunk(&chunk, "test chunk");

    println!("===\n\n");

    let mut vm = VM::new(&chunk);
    vm.run().unwrap();
}
