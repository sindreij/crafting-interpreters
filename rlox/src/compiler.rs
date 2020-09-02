use num_enum::{IntoPrimitive, TryFromPrimitive};

use log::trace;

use crate::{
    chunk::{Chunk, OpCode},
    debug::disassemble_chunk,
    object::{ObjFunction, ObjHeap, ObjKind},
    scanner::{Scanner, Token, TokenType},
    value::Value,
};
use std::{convert::TryInto, mem};

struct Parser<'a> {
    current: Token<'a>,
    previous: Token<'a>,
    scanner: Scanner<'a>,
    heap: &'a mut ObjHeap,
    had_error: bool,
    panic_mode: bool,
    compiler: Compiler<'a>,
}

#[derive(Eq, PartialEq)]
enum FunctionType {
    Function,
    Script,
}

struct Compiler<'a> {
    function: ObjFunction,
    function_type: FunctionType,

    locals: Vec<Local<'a>>,
    scope_depth: i32,
}

impl<'a> Compiler<'a> {
    fn new(function_type: FunctionType, name: Option<String>) -> Compiler<'a> {
        let mut function = ObjFunction::new();

        if function_type != FunctionType::Script {
            function.name = name;
        }

        let local = Local {
            depth: 0,
            name: Token {
                line: 0,
                str: "",
                typ: TokenType::Identifier,
            },
        };

        let mut locals = Vec::with_capacity(256);
        locals.push(local);

        Compiler {
            function,
            function_type,
            locals,
            scope_depth: 0,
        }
    }

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

struct Local<'a> {
    name: Token<'a>,
    depth: i32,
}

pub fn compile(source: &str, heap: &mut ObjHeap) -> Result<ObjFunction, ()> {
    let scanner = Scanner::new(source);
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
        heap,
        compiler: Compiler::new(FunctionType::Script, None),
    };
    let function = parser.compile()?;

    Ok(function)
}

impl<'a> Parser<'a> {
    fn compile(&mut self) -> Result<ObjFunction, ()> {
        self.advance();

        while !self.match_token(TokenType::EOF) {
            self.declaration();
        }

        let function = self.end_compiler();

        if self.had_error {
            Err(())
        } else {
            Ok(function)
        }
    }

    fn end_compiler(&mut self) -> ObjFunction {
        self.emit_return();

        let function = self.compiler.function.clone();

        if std::env::var("PRINT_CODE").is_ok() {
            if !self.had_error {
                let heap = self.heap.clone();
                disassemble_chunk(
                    self.current_chunk(),
                    function.name.as_deref().unwrap_or("<script>"),
                    &heap,
                );
            }
        }

        return function;
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
        if self.compiler.scope_depth == 0 {
            return;
        }
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
        if self.match_token(TokenType::Fun) {
            self.fun_declaration();
        } else if self.match_token(TokenType::Var) {
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
        } else if self.match_token(TokenType::For) {
            self.for_statement();
        } else if self.match_token(TokenType::If) {
            self.if_statement();
        } else if self.match_token(TokenType::While) {
            self.while_statement();
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

    fn fun_declaration(&mut self) {
        let global = self.parse_variable("Expect function name");

        self.mark_initialized();
        self.function(FunctionType::Function);
        self.define_variable(global);
    }

    fn function(&mut self, function_type: FunctionType) {
        let mut compiler = Compiler::new(function_type, Some(self.previous.str.to_owned()));
        // This is not the way the book is doing it. Let's see if it works out. If not
        // we need to use the enclosing-thing. See
        // https://craftinginterpreters.com/calls-and-functions.html#function-declarations
        mem::swap(&mut self.compiler, &mut compiler);
        self.begin_scope();

        self.consume(TokenType::LeftParen, "Expect '(' after function name");

        if !self.check(TokenType::RightParen) {
            loop {
                self.compiler.function.arity += 1;
                if self.compiler.function.arity > 255 {
                    self.error_at_current("Cannot have more than 255 parameters");
                }

                let param_constant = self.parse_variable("Expect parameter name");
                self.define_variable(param_constant);

                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
        }

        println!("{:?}", self.current);

        self.consume(TokenType::RightParen, "Expect ')' after parameters");

        self.consume(TokenType::LeftBrace, "Expect '{' before function body");
        self.block();

        let function = self.end_compiler();
        mem::swap(&mut self.compiler, &mut compiler);
        let function = self.heap.allocate_obj(ObjKind::Function(function));
        let function_constant = self.make_constant(Value::Obj(function));
        self.emit_opcode_byte(OpCode::Constant, function_constant);
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

    fn if_statement(&mut self) {
        self.consume(TokenType::LeftParen, "Expect '(' after 'if'");
        self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after condition");

        let then_jump = self.emit_jump(OpCode::JumpIfFalse);
        self.emit_opcode(OpCode::Pop);
        self.statement();

        let else_jump = self.emit_jump(OpCode::Jump);

        self.patch_jump(then_jump);
        self.emit_opcode(OpCode::Pop);

        if self.match_token(TokenType::Else) {
            self.statement();
        }
        self.patch_jump(else_jump);
    }

    fn while_statement(&mut self) {
        let loop_start = self.current_chunk().code.len();
        self.consume(TokenType::LeftParen, "Expect '(' after while");
        self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after condition");

        let exit_jump = self.emit_jump(OpCode::JumpIfFalse);

        self.emit_opcode(OpCode::Pop);
        self.statement();
        self.emit_loop(loop_start);
        self.patch_jump(exit_jump);
        self.emit_opcode(OpCode::Pop);
    }

    fn for_statement(&mut self) {
        self.begin_scope();
        self.consume(TokenType::LeftParen, "Expect '(' after 'for'");

        if self.match_token(TokenType::Semicolon) {
            // No initializer
        } else if self.match_token(TokenType::Var) {
            self.var_declaration();
        } else {
            self.expression_statement();
        }

        let mut loop_start = self.current_chunk().code.len();

        let exit_jump = if !self.match_token(TokenType::Semicolon) {
            self.expression();
            self.consume(TokenType::Semicolon, "Expect ';' after loop condition");
            let exit_jump = self.emit_jump(OpCode::JumpIfFalse);
            Some(exit_jump)
        } else {
            None
        };

        if !self.match_token(TokenType::RightParen) {
            let body_jump = self.emit_jump(OpCode::Jump);
            let increment_start = self.current_chunk().code.len();
            self.expression();
            self.emit_opcode(OpCode::Pop);

            self.consume(TokenType::RightParen, "Expect ')' after for clauses");

            self.emit_loop(loop_start);

            loop_start = increment_start;
            self.patch_jump(body_jump);
        }
        self.statement();

        self.emit_loop(loop_start);

        if let Some(exit_jump) = exit_jump {
            self.patch_jump(exit_jump);
            self.emit_opcode(OpCode::Pop);
        }

        self.end_scope();
    }

    fn emit_jump(&mut self, instruction: OpCode) -> usize {
        self.emit_opcode(instruction);
        self.emit_byte(0xff);
        self.emit_byte(0xff);
        self.current_chunk().code.len() - 2
    }

    fn emit_loop(&mut self, loop_start: usize) {
        self.emit_opcode(OpCode::Loop);
        let offset = self.current_chunk().code.len() - loop_start + 2;
        if offset > 0xffff {
            self.error("Loop body too large");
        }

        self.emit_byte(((offset >> 8) & 0xff) as u8);
        self.emit_byte((offset & 0xff) as u8);
    }

    fn patch_jump(&mut self, offset: usize) {
        // -2 to adjust for the bytecode for the jump offset itself.
        let jump = self.current_chunk().code.len() - offset - 2;

        if jump > 0xffff {
            self.error("Too much code to jump over");
        }

        self.current_chunk().code[offset] = ((jump >> 8) & 0xff) as u8;
        self.current_chunk().code[offset + 1] = (jump & 0xff) as u8;
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

    fn and(&mut self, _can_assign: bool) {
        let end_jump = self.emit_jump(OpCode::JumpIfFalse);
        self.emit_opcode(OpCode::Pop);
        self.parse_precedence(Precedence::And);
        self.patch_jump(end_jump);
    }

    fn or(&mut self, _can_assign: bool) {
        let else_jump = self.emit_jump(OpCode::JumpIfFalse);
        let end_jump = self.emit_jump(OpCode::Jump);

        self.patch_jump(else_jump);
        self.emit_opcode(OpCode::Pop);

        self.parse_precedence(Precedence::Or);
        self.patch_jump(end_jump);
    }

    fn call(&mut self, _can_assign: bool) {
        let arg_count = self.argument_list();
        self.emit_opcode_byte(OpCode::Call, arg_count);
    }

    fn argument_list(&mut self) -> u8 {
        let mut arg_count = 0;

        if !self.check(TokenType::RightParen) {
            loop {
                self.expression();

                if arg_count == 255 {
                    self.error("Cannot have more than 255 arguments.");
                }
                arg_count += 1;
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
        }

        self.consume(TokenType::RightParen, "Expect ')' after arguments");

        arg_count
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
        &mut self.compiler.function.chunk
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
            infix: Some(Parser::call),
            precedence: Precedence::Call,
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
            infix: Some(Parser::and),
            precedence: Precedence::And,
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
            infix: Some(Parser::or),
            precedence: Precedence::Or,
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
