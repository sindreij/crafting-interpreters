use crate::scanner::{Scanner, TokenType};

pub fn compile(source: &str) {
    let mut scanner = Scanner::new(source);

    let mut line = None;

    loop {
        let token = scanner.scan_token();
        if Some(token.line) != line {
            print!("{:4}", token.line);
            line = Some(token.line);
        } else {
            print!("   | ");
        }

        println!("{:12?} '{:?}'", token.typ, token.str);

        if token.typ == TokenType::EOF {
            break;
        }
    }
}
