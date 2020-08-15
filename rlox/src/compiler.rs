use num_enum::{IntoPrimitive, TryFromPrimitive};

use log::trace;

use crate::{
    chunk::{Chunk, OpCode},
    debug::disassemble_chunk,
    object::ObjHeap,
    scanner::{Scanner, Token, TokenType},
    value::Value,
};
use std::convert::TryInto;

struct Parser<'a> {
    current: Token<'a>,
    previous: Token<'a>,
    scanner: Scanner<'a>,
    heap: &'a mut ObjHeap,
    had_error: bool,
    panic_mode: bool,
    compiling_chunk: &'a mut Chunk,
    compiler: Compiler<'a>,
}

struct Compiler<'a> {
    locals: Vec<Local<'a>>,
    scope_depth: i32,
}

struct Local<'a> {
    name: Token<'a>,
    depth: i32,
}

pub fn compile(source: &str, heap: &mut ObjHeap) -> Result<Chunk, ()> {
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
        heap,
        compiler: Compiler {
            locals: Vec::with_capacity(256),
            scope_depth: 0,
        },
    };
    parser.compile()?;

    Ok(chunk)
}

impl<'a> Parser<'a> {
    fn compile(&mut self) -> Result<(), ()> {
        self.advance();

        while !self.match_token(TokenType::EOF) {
            self.declaration();
        }

        self.end_compiler();

        if self.had_error {
            Err(())
        } else {
            Ok(())
        }
    }

    fn end_compiler(&mut self) {
        self.emit_return();

        if std::env::var("PRINT_CODE").is_ok() {
            if !self.had_error {
                let heap = self.heap.clone();
                disassemble_chunk(self.current_chunk(), "code", &heap);
            }
        }
    }

    fn begin_scope(&mut self) {
        self.compiler.scope_depth += 1;
    }

    fn end_scope(&mut self) {
        self.compiler.scope_depth -= 1;

        while self.compiler.locals.len() > 0
            && self.compiler.locals.last().unwrap().depth > self.compiler.scope_depth
        {
            self.emit_opcode(OpCode::Pop);
            self.compiler.locals.pop();
        }
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

    fn parse_precedence(&mut self, precedence: Precedence) {
        self.advance();
        let prefix_rule = get_rule(self.previous.typ).prefix;

        let prefix_rule = match prefix_rule {
            None => {
                self.error("Expect expression");
                return;
            }
            Some(rule) => rule,
        };

        let can_assign = precedence <= Precedence::Assignment;
        prefix_rule(self, can_assign);

        while precedence <= get_rule(self.current.typ).precedence {
            self.advance();
            let infix_rule = get_rule(self.previous.typ).infix.unwrap();
            infix_rule(self, can_assign);
        }

        if can_assign && self.match_token(TokenType::Equal) {
            self.error("Invalid assignment target");
        }
    }

    fn identifier_constant(&mut self, name: Token) -> u8 {
        let string = self.heap.copy_string(name.str);
        self.make_constant(Value::Obj(string))
    }

    fn parse_variable(&mut self, error_message: &'static str) -> u8 {
        self.consume(TokenType::Identifier, error_message);

        self.declare_variable();
        if self.compiler.scope_depth > 0 {
            0
        } else {
            self.identifier_constant(self.previous)
        }
    }

    fn declare_variable(&mut self) {
        if self.compiler.scope_depth == 0 {
            // Global variables are implicitly declared.
            return;
        }

        let name = self.previous;

        let mut exists = false;
        for local in self.compiler.locals.iter().rev() {
            if local.depth != -1 && local.depth < self.compiler.scope_depth {
                break;
            }

            if local.name.str == name.str {
                exists = true;
                break;
            }
        }

        if exists {
            self.error("Variable with this name already declared in this scope");
        }

        self.add_local(name);
    }

    fn mark_initialized(&mut self) {
        self.compiler.locals.last_mut().unwrap().depth = self.compiler.scope_depth;
    }

    fn define_variable(&mut self, global: u8) {
        if self.compiler.scope_depth > 0 {
            // No need to define the local variable. It's already on the stack, exactly where
            // we want it to be
            self.mark_initialized();
            return;
        }

        self.emit_opcode_byte(OpCode::DefineGlobal, global);
    }

    fn add_local(&mut self, name: Token<'a>) {
        if self.compiler.locals.len() == 256 {
            self.error("Too many local variables in function");
            return;
        }

        self.compiler.locals.push(Local { name, depth: -1 })
    }

    fn declaration(&mut self) {
        if self.match_token(TokenType::Var) {
            self.var_declaration();
        } else {
            self.statement();
        }

        if self.panic_mode {
            self.synchronize();
        }
    }

    fn statement(&mut self) {
        if self.match_token(TokenType::Print) {
            self.print_statement();
        } else if self.match_token(TokenType::LeftBrace) {
            self.begin_scope();
            self.block();
            self.end_scope();
        } else {
            self.expression_statement();
        }
    }

    fn expression(&mut self) {
        self.parse_precedence(Precedence::Assignment);
    }

    fn block(&mut self) {
        while !self.check(TokenType::RightBrace) && !self.check(TokenType::EOF) {
            self.declaration();
        }
        self.consume(TokenType::RightBrace, "Expect '{' after block");
    }

    fn var_declaration(&mut self) {
        let global = self.parse_variable("Expect variable name");

        if self.match_token(TokenType::Equal) {
            self.expression();
        } else {
            self.emit_opcode(OpCode::Nil);
        }

        self.consume(
            TokenType::Semicolon,
            "Expect ';' after variable declaration",
        );

        self.define_variable(global);
    }

    fn expression_statement(&mut self) {
        self.expression();
        self.consume(TokenType::Semicolon, "Expect ';' after expression");
        self.emit_opcode(OpCode::Pop);
    }

    fn print_statement(&mut self) {
        self.expression();
        self.consume(TokenType::Semicolon, "Expect ';' after value");
        self.emit_opcode(OpCode::Print);
    }

    fn synchronize(&mut self) {
        self.panic_mode = false;
        while self.current.typ != TokenType::EOF {
            if self.previous.typ == TokenType::Semicolon {
                return;
            }

            match self.current.typ {
                TokenType::Class
                | TokenType::Fun
                | TokenType::Var
                | TokenType::For
                | TokenType::If
                | TokenType::While
                | TokenType::Print
                | TokenType::Return => return,

                _ => { /* Do nothing */ }
            }
            self.advance();
        }
    }

    fn number(&mut self, _can_assign: bool) {
        trace!("Number");
        let value = self.previous.str.parse::<f64>().unwrap();
        self.emit_constant(Value::Number(value));
    }

    fn grouping(&mut self, _can_assign: bool) {
        trace!("Grouping");
        self.expression();
        self.consume(TokenType::RightParen, "Expected ')' after expression");
        trace!("Grouping FIN");
    }

    fn unary(&mut self, _can_assign: bool) {
        trace!("Unary");
        let operator_type = self.previous.typ;

        self.parse_precedence(Precedence::Unary);

        match operator_type {
            TokenType::Minus => self.emit_opcode(OpCode::Negate),
            TokenType::Bang => self.emit_opcode(OpCode::Not),
            TokenType::Plus => {
                // Unary + don't actually do anything, but we'll allow it
            }
            _ => unreachable!(),
        };
        trace!("Unary FIN");
    }

    fn binary(&mut self, _can_assign: bool) {
        // Remember the operator
        let operator_type = self.previous.typ;
        trace!("Binary {:?}", operator_type);

        // Compile the right operand
        let rule = get_rule(operator_type);
        self.parse_precedence((u8::from(rule.precedence) + 1).try_into().unwrap());

        match operator_type {
            TokenType::Plus => self.emit_opcode(OpCode::Add),
            TokenType::Minus => self.emit_opcode(OpCode::Subtract),
            TokenType::Star => self.emit_opcode(OpCode::Multiply),
            TokenType::Slash => self.emit_opcode(OpCode::Divide),

            TokenType::BangEqual => self.emit_opcodes(OpCode::Equal, OpCode::Not),
            TokenType::EqualEqual => self.emit_opcode(OpCode::Equal),
            TokenType::Greater => self.emit_opcode(OpCode::Greater),
            TokenType::GreaterEqual => self.emit_opcodes(OpCode::Less, OpCode::Not),
            TokenType::Less => self.emit_opcode(OpCode::Less),
            TokenType::LessEqual => self.emit_opcodes(OpCode::Greater, OpCode::Not),

            _ => unreachable!(),
        };
        trace!("Binary {:?} FIN", operator_type);
    }

    fn literal(&mut self, _can_assign: bool) {
        match self.previous.typ {
            TokenType::False => self.emit_opcode(OpCode::False),
            TokenType::True => self.emit_opcode(OpCode::True),
            TokenType::Nil => self.emit_opcode(OpCode::Nil),
            _ => unreachable!(),
        }
    }

    fn string(&mut self, _can_assign: bool) {
        let constant = Value::Obj(
            self.heap
                .copy_string(&self.previous.str[1..self.previous.str.len() - 1]),
        );

        self.emit_constant(constant);
    }

    fn variable(&mut self, can_assign: bool) {
        self.named_variable(self.previous, can_assign);
    }

    fn named_variable(&mut self, name: Token, can_assign: bool) {
        let (local_arg, error) = self.compiler.resolve_local(name);

        // I try to make how we do error handling match how it's done in the book. However this is
        // an edge case where that is difficult because of borrowing, so we move the call to
        // self.error here
        if let Some(error) = error {
            self.error(error)
        }

        let (arg, get_opt, set_opt) = if let Some(local_arg) = local_arg {
            (local_arg, OpCode::GetLocal, OpCode::SetLocal)
        } else {
            (
                self.identifier_constant(name),
                OpCode::GetGlobal,
                OpCode::SetGlobal,
            )
        };

        if can_assign && self.match_token(TokenType::Equal) {
            self.expression();
            self.emit_opcode_byte(set_opt, arg);
        } else {
            self.emit_opcode_byte(get_opt, arg);
        }
    }

    fn consume(&mut self, typ: TokenType, message: &'static str) {
        if self.current.typ == typ {
            self.advance();
            return;
        }
        self.error_at_current(message);
    }

    fn match_token(&mut self, typ: TokenType) -> bool {
        if !self.check(typ) {
            return false;
        }
        self.advance();
        return true;
    }

    fn check(&self, typ: TokenType) -> bool {
        self.current.typ == typ
    }

    fn emit_byte(&mut self, byte: u8) {
        let line = self.previous.line;
        self.current_chunk().write(byte, line);
    }

    fn emit_opcode(&mut self, opcode: OpCode) {
        self.emit_byte(opcode as u8);
    }

    fn emit_opcodes(&mut self, opcode: OpCode, opcode2: OpCode) {
        self.emit_opcode(opcode);
        self.emit_opcode(opcode2);
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

impl<'a> Compiler<'a> {
    fn resolve_local(&self, name: Token) -> (Option<u8>, Option<&'static str>) {
        let mut error = None;
        for (i, local) in self.locals.iter().enumerate().rev() {
            if local.name.str == name.str {
                if local.depth == -1 {
                    error = Some("Cannot read local variable in its own initializer");
                }
                return (Some(i.try_into().unwrap()), error);
            }
        }

        (None, error)
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
            prefix: Some(Parser::unary),
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
            prefix: Some(Parser::unary),
            infix: None,
            precedence: Precedence::None,
        },
        BangEqual => ParseRule {
            prefix: None,
            infix: Some(Parser::binary),
            precedence: Precedence::Equality,
        },
        Equal => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        EqualEqual => ParseRule {
            prefix: None,
            infix: Some(Parser::binary),
            precedence: Precedence::Equality,
        },
        Greater => ParseRule {
            prefix: None,
            infix: Some(Parser::binary),
            precedence: Precedence::Comparison,
        },
        GreaterEqual => ParseRule {
            prefix: None,
            infix: Some(Parser::binary),
            precedence: Precedence::Comparison,
        },
        Less => ParseRule {
            prefix: None,
            infix: Some(Parser::binary),
            precedence: Precedence::Comparison,
        },
        LessEqual => ParseRule {
            prefix: None,
            infix: Some(Parser::binary),
            precedence: Precedence::Comparison,
        },
        Identifier => ParseRule {
            prefix: Some(Parser::variable),
            infix: None,
            precedence: Precedence::None,
        },
        String => ParseRule {
            prefix: Some(Parser::string),
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
            prefix: Some(Parser::literal),
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
            prefix: Some(Parser::literal),
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
            prefix: Some(Parser::literal),
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
type ParserFn<'a> = fn(&mut Parser<'a>, bool);

struct ParseRule<'a> {
    prefix: Option<ParserFn<'a>>,
    infix: Option<ParserFn<'a>>,
    precedence: Precedence,
}
