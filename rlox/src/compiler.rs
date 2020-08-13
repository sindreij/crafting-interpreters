use num_enum::{IntoPrimitive, TryFromPrimitive};

use crate::{
    chunk::{Chunk, OpCode},
    scanner::{Scanner, Token, TokenType},
    value::Value,
};
use std::convert::TryInto;

struct Parser<'a> {
    current: Token<'a>,
    previous: Token<'a>,
    scanner: Scanner<'a>,
    had_error: bool,
    panic_mode: bool,
    compiling_chunk: &'a mut Chunk,
}

pub fn compile(source: &str) -> Result<Chunk, ()> {
    let scanner = Scanner::new(source);
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

    fn parse_precedence(&mut self, precendence: Precedence) {
        self.advance();
        let prefix_rule = get_rule(self.previous.typ).prefix;

        let prefix_rule = match prefix_rule {
            None => {
                self.error("Expect expression");
                return;
            }
            Some(rule) => rule,
        };

        prefix_rule(self);

        while precendence <= get_rule(self.current.typ).precedence {
            self.advance();
            let infix_rule = get_rule(self.previous.typ).infix.unwrap();
            infix_rule(self);
        }
    }

    fn expression(&mut self) {
        self.parse_precedence(Precedence::Assignment);
    }

    fn number(&mut self) {
        let value = self.previous.str.parse::<f64>().unwrap();
        self.emit_constant(Value::Number(value));
    }

    fn grouping(&mut self) {
        self.expression();
        self.consume(TokenType::RightParen, "Expected ')' after expression");
    }

    fn unary(&mut self) {
        let operator_type = self.previous.typ;

        self.parse_precedence(Precedence::Unary);

        match operator_type {
            TokenType::Minus => self.emit_opcode(OpCode::Negate),
            _ => unreachable!(),
        };
    }

    fn binary(&mut self) {
        // Remember the operator
        let operator_type = self.previous.typ;

        // Compile the right operand
        let rule = get_rule(operator_type);
        self.parse_precedence((u8::from(rule.precedence) + 1).try_into().unwrap());

        match operator_type {
            TokenType::Plus => self.emit_opcode(OpCode::Add),
            TokenType::Minus => self.emit_opcode(OpCode::Subtract),
            TokenType::Star => self.emit_opcode(OpCode::Multiply),
            TokenType::Slash => self.emit_opcode(OpCode::Divide),
            _ => unreachable!(),
        };
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

    fn emit_return(&mut self) {
        self.emit_opcode(OpCode::Return);
    }

    fn emit_opcode_byte(&mut self, opcode: OpCode, byte: u8) {
        self.emit_opcode(opcode);
        self.emit_byte(byte);
    }

    fn emit_constant(&mut self, value: Value) {
        let constant = self.make_constant(value);
        self.emit_opcode_byte(OpCode::Constant, constant);
    }

    fn make_constant(&mut self, value: Value) -> u8 {
        self.current_chunk().add_constant(value)
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
        self.panic_mode = true;
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

#[derive(Debug, Clone, Copy, IntoPrimitive, TryFromPrimitive, Ord, PartialOrd, Eq, PartialEq)]
#[repr(u8)]
enum Precedence {
    None,
    Assignment, // =
    Or,         // or
    And,        // and
    Equality,   // == !=
    Comparison, // < > <= >=
    Term,       // + -
    Factor,     // * /
    Unary,      // ! -
    Call,       // . ()
    Primary,
}

fn get_rule<'a>(typ: TokenType) -> ParseRule<'a> {
    use TokenType::*;

    match typ {
        LeftParen => ParseRule {
            prefix: Some(Parser::grouping),
            infix: None,
            precedence: Precedence::None,
        },
        RightParen => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        LeftBrace => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        RightBrace => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        Comma => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        Dot => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        Minus => ParseRule {
            prefix: Some(Parser::unary),
            infix: Some(Parser::binary),
            precedence: Precedence::Term,
        },
        Plus => ParseRule {
            prefix: None,
            infix: Some(Parser::binary),
            precedence: Precedence::Term,
        },
        Semicolon => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        Slash => ParseRule {
            prefix: None,
            infix: Some(Parser::binary),
            precedence: Precedence::Factor,
        },
        Star => ParseRule {
            prefix: None,
            infix: Some(Parser::binary),
            precedence: Precedence::Factor,
        },
        Bang => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        BangEqual => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        Equal => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        EqualEqual => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        Greater => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        GreaterEqual => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        Less => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        LessEqual => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        Identifier => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        String => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        Number => ParseRule {
            prefix: Some(Parser::number),
            infix: None,
            precedence: Precedence::None,
        },
        And => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        Class => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        Else => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        False => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        For => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        Fun => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        If => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        Nil => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        Or => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        Print => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        Return => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        Super => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        This => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        True => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        Var => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        While => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        Error => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        EOF => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        NOOP => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
    }
}

// type ParserFn<'a> = Box<dyn Fn(&mut Parser<'a>)>;
type ParserFn<'a> = fn(&mut Parser<'a>);

struct ParseRule<'a> {
    prefix: Option<ParserFn<'a>>,
    infix: Option<ParserFn<'a>>,
    precedence: Precedence,
}
