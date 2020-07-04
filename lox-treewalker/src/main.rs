use std::io::prelude::*;

use anyhow::Result;

mod scanner;

fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();

    if args.len() > 2 {
        eprintln!("Usage: lox-treewalker [script]");
        Ok(())
    } else if args.len() == 2 {
        Lox::new().run_file(&args[1])
    } else {
        Lox::new().run_prompt()
    }
}

struct Lox {
    had_error: bool,
}

impl Lox {
    fn new() -> Lox {
        Lox { had_error: false }
    }

    fn run_file(&mut self, name: &str) -> Result<()> {
        let mut file = std::fs::File::open(name)?;
        let mut buffer = String::new();
        file.read_to_string(&mut buffer)?;

        if self.had_error {
            std::process::exit(65);
        }

        self.run(&buffer)
    }

    fn run_prompt(&mut self) -> Result<()> {
        let mut buffer = String::new();
        let mut stdout = std::io::stdout();
        let stdin = std::io::stdin();
        loop {
            stdout.write(b"> ")?;
            stdout.flush()?;

            buffer.clear();
            stdin.read_line(&mut buffer)?;
            if buffer.is_empty() {
                break;
            }
            self.run(&buffer)?;
            // If the user makes a mistake, it shouldn’t kill their entire session:
            self.had_error = false;
        }

        Ok(())
    }

    fn run(&mut self, source: &str) -> Result<()> {
        let scanner = scanner::Scanner::new(source);
        let tokens = scanner.scanTokens();

        for token in tokens {
            println!("{:?}", token);
        }

        Ok(())
    }

    fn error(&mut self, line: u32, message: &str) {
        self.report(line, "", message);
    }

    fn report(&mut self, line: u32, location: &str, message: &str) {
        println!(
            "[line {line}] Error{location}: {message}",
            line = line,
            location = location,
            message = message
        );
        self.had_error = true;
    }
}
