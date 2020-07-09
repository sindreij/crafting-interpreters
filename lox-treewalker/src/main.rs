use std::io::prelude::*;

use anyhow::Result;

use error_reporter::ErrorReporter;
use interpreter::Interpreter;
use parser::Parser;
use resolver::Resolver;
use runtime_error::RuntimeError;

mod ast;
mod environment;
mod error_reporter;
mod interpreter;
mod parser;
mod resolver;
mod runtime_error;
mod scanner;
mod token;
mod value;

fn main() -> Result<()> {
    // let expr = ast::Expr::Binary {
    //     left: Box::new(ast::Expr::Unary {
    //         operator: token::Token::new(token::TokenType::Minus, "-".to_owned(), 1),
    //         right: Box::new(ast::Expr::Literal(ast::Literal::Number(123.))),
    //     }),
    //     operator: token::Token::new(token::TokenType::Star, "*".to_owned(), 1),
    //     right: Box::new(ast::Expr::Grouping(Box::new(ast::Expr::Literal(
    //         ast::Literal::Number(45.67),
    //     )))),
    // };

    // println!("{}", expr);

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
    interpreter: Interpreter,
}

impl Lox {
    fn new() -> Lox {
        Lox {
            interpreter: Interpreter::new(),
        }
    }

    fn run_file(&mut self, name: &str) -> Result<()> {
        let mut file = std::fs::File::open(name)?;
        let mut buffer = String::new();
        file.read_to_string(&mut buffer)?;

        let mut errors = ErrorReporter { had_error: false };

        let result = self.run(&buffer, &mut errors);

        match result {
            Ok(()) => {}
            Err(RunError::ParseError) => {
                std::process::exit(65);
            }
            Err(RunError::TokenizeError) => {
                std::process::exit(65);
            }
            Err(RunError::RuntimeError(error)) => {
                println!("{}", error);
                std::process::exit(70);
            }
        }

        Ok(())
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
            let mut errors = ErrorReporter { had_error: false };
            if let Err(err) = self.run(&buffer, &mut errors) {
                // If the user makes a mistake, it shouldn’t kill their entire session:
                println!("{}", err);
            }
            // If the user makes a mistake, it shouldn’t kill their entire session:
            errors.had_error = false;
        }

        Ok(())
    }

    fn run(&mut self, source: &str, errors: &mut ErrorReporter) -> Result<(), RunError> {
        let mut scanner = scanner::Scanner::new(source, errors);
        let tokens = scanner.scan_tokens();

        if errors.had_error {
            return Err(RunError::TokenizeError);
        }

        let parser = Parser::new(tokens);
        let statements = parser.parse();

        if errors.had_error {
            return Err(RunError::ParseError);
        }

        match statements {
            Some(statements) => {
                let mut resolver = Resolver::new(&mut self.interpreter, errors);
                resolver.resolve(&statements);

                if errors.had_error {
                    return Err(RunError::ParseError);
                }

                self.interpreter
                    .interpret(&statements)
                    .map_err(|err| RunError::RuntimeError(err))?;
            }
            None => return Err(RunError::ParseError),
        }

        Ok(())
    }
}

#[derive(Debug)]
enum RunError {
    TokenizeError,
    ParseError,
    RuntimeError(RuntimeError),
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunError::TokenizeError => write!(f, "Error tokenizing"),
            RunError::ParseError => write!(f, "Error parsing"),
            RunError::RuntimeError(inner) => write!(f, "{}", inner),
        }
    }
}

impl std::error::Error for RunError {}
