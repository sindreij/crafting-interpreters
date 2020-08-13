use anyhow::Result;

use std::io::{Read, Write};
use vm::{InterpretError, VM};

mod chunk;
mod compiler;
mod debug;
mod scanner;
mod value;
mod vm;

// fn main() {
//     let mut chunk = Chunk::new();

//     let constant = chunk.add_constant(Value::Number(1.2));
//     chunk.write_op(OpCode::Constant, 123);
//     chunk.write(constant, 123);

//     let constant = chunk.add_constant(Value::Number(3.4));
//     chunk.write_op(OpCode::Constant, 123);
//     chunk.write(constant, 123);

//     chunk.write_op(OpCode::Add, 123);

//     let constant = chunk.add_constant(Value::Number(5.6));
//     chunk.write_op(OpCode::Constant, 123);
//     chunk.write(constant, 123);

//     chunk.write_op(OpCode::Divide, 123);

//     chunk.write_op(OpCode::Negate, 123);

//     chunk.write_op(OpCode::Return, 123);

//     let mut vm = Box::new(VM::new(&chunk));
//     vm.run().unwrap();
// }

fn main() -> Result<()> {
    pretty_env_logger::init();

    let args = std::env::args().collect::<Vec<_>>();

    if args.len() == 1 {
        repl()?;
    } else if args.len() == 2 {
        run_file(&args[1])?;
    } else {
        eprintln!("Usage: {} [path]\n", args[0]);
        std::process::exit(64);
    }
    Ok(())
}

fn repl() -> Result<()> {
    let mut buffer = String::new();
    let mut stdout = std::io::stdout();
    let mut vm = VM::new();
    let stdin = std::io::stdin();
    loop {
        stdout.write(b"> ")?;
        stdout.flush()?;
        buffer.clear();

        stdin.read_line(&mut buffer)?;

        if buffer.is_empty() {
            stdout.write(b"\n")?;
            stdout.flush()?;
            break;
        }

        if let Err(err) = vm.interpret(&buffer) {
            eprintln!("{}", err);
        }
    }

    Ok(())
}

fn run_file(name: &str) -> Result<()> {
    let mut file = std::fs::File::open(name)?;
    let mut buffer = String::new();
    file.read_to_string(&mut buffer)?;

    let mut vm = VM::new();
    let result = vm.interpret(&buffer);

    if let Err(err) = result {
        match err {
            InterpretError::CompileError => std::process::exit(65),
            InterpretError::RuntimeError(inner) => {
                eprintln!("Runtime Error: {}", inner);
                std::process::exit(70)
            }
        }
    }

    Ok(())
}
