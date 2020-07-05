use std::io::prelude::*;

use anyhow::Result;

use error_reporter::ErrorReporter;
use parser::Parser;

mod ast;
mod error_reporter;
mod parser;
mod scanner;
mod token;

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

struct Lox {}

impl Lox {
    fn new() -> Lox {
        Lox {}
    }

    fn run_file(&mut self, name: &str) -> Result<()> {
        let mut file = std::fs::File::open(name)?;
        let mut buffer = String::new();
        file.read_to_string(&mut buffer)?;

        let mut errors = ErrorReporter { had_error: false };

        self.run(&buffer, &mut errors)?;

        if errors.had_error {
            std::process::exit(65);
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
            self.run(&buffer, &mut errors)?;
            // If the user makes a mistake, it shouldn’t kill their entire session:
            errors.had_error = false;
        }

        Ok(())
    }

    fn run(&mut self, source: &str, errors: &mut ErrorReporter) -> Result<()> {
        let mut scanner = scanner::Scanner::new(source, errors);
        let tokens = scanner.scan_tokens();

        let parser = Parser::new(tokens);
        let expression = parser.parse();

        if let Some(expression) = expression {
            println!("{}", expression)
        }

        Ok(())
    }
}
