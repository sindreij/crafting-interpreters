use crate::{
    chunk::{Chunk, OpCode},
    scanner::{Scanner, Token, TokenType},
};

struct Parser<'a> {
    current: Token<'a>,
    previous: Token<'a>,
    scanner: Scanner<'a>,
    had_error: bool,
    panic_mode: bool,
    compiling_chunk: &'a mut Chunk,
}

pub fn compile(source: &str) -> Result<Chunk, ()> {
    let mut scanner = Scanner::new(source);
    let mut chunk = Chunk::new();
    let mut parser = Parser {
        // Add some tokens so that we can create a parser. This will soon be overwritten
        current: Token {
            typ: TokenType::NOOP,
            str: "",
            line: 1,
        },
        previous: Token {
            typ: TokenType::NOOP,
            str: "",
            line: 1,
        },
        scanner,
        had_error: false,
        panic_mode: false,
        compiling_chunk: &mut chunk,
    };
    parser.compile()?;

    Ok(chunk)
}

impl<'a> Parser<'a> {
    fn compile(&mut self) -> Result<(), ()> {
        self.advance();
        self.expression();
        self.consume(TokenType::EOF, "Expected end of expression");

        self.end_compiler();

        if self.had_error {
            Err(())
        } else {
            Ok(())
        }
    }

    fn end_compiler(&mut self) {
        self.emit_return();
    }

    fn emit_return(&mut self) {
        self.emit_opcode(OpCode::Return);
    }

    fn advance(&mut self) {
        // parser.previous = parser.current;
        // will also do the reverse (set current to previous), but that is ok since
        // we will soon replace current
        std::mem::swap(&mut self.previous, &mut self.current);

        loop {
            self.current = self.scanner.scan_token();
            if self.current.typ != TokenType::Error {
                break;
            }
            self.error_at_current(self.current.str);
        }
    }

    fn expression(&mut self) {
        todo!()
    }

    fn consume(&mut self, typ: TokenType, message: &'static str) {
        if self.current.typ == typ {
            self.advance();
            return;
        }
        self.error_at_current(message);
    }

    fn emit_byte(&mut self, byte: u8) {
        let line = self.previous.line;
        self.current_chunk().write(byte, line);
    }

    fn emit_opcode(&mut self, opcode: OpCode) {
        self.emit_byte(opcode as u8);
    }

    fn emit_opcode_byte(&mut self, opcode: OpCode, byte: u8) {
        self.emit_opcode(opcode);
        self.emit_byte(byte);
    }

    fn current_chunk(&mut self) -> &mut Chunk {
        self.compiling_chunk
    }

    fn error_at_current(&mut self, message: &str) {
        self.error_at(self.current, message);
    }

    fn error(&mut self, message: &str) {
        self.error_at(self.previous, message);
    }

    fn error_at(&mut self, token: Token, message: &str) {
        if self.panic_mode {
            return;
        }
        eprint!("[line {}] Error", token.line);
        match token.typ {
            TokenType::EOF => eprint!(" at end"),
            TokenType::Error => {}
            _ => eprint!(" at '{}'", token.str),
        };
        eprintln!(": {}", message);
        self.had_error = true;
    }
}
