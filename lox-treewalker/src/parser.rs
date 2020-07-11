use crate::{
    ast::{Expr, Literal, Stmt, StmtFunction},
    error_reporter::format_err,
    token::{Token, TokenType},
};
use std::sync::atomic::{AtomicUsize, Ordering};

static EXPR_COUNTER: AtomicUsize = AtomicUsize::new(1);

fn next_expr_id() -> usize {
    EXPR_COUNTER.fetch_add(1, Ordering::Relaxed)
}

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

#[derive(Debug)]
struct ParseError {
    token: Token,
    message: String,
}

impl ParseError {
    fn new(token: Token, message: impl Into<String>) -> ParseError {
        ParseError {
            token,
            message: message.into(),
        }
    }
}

type Result<T> = std::result::Result<T, ParseError>;

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.token.typ == TokenType::EOF {
            write!(
                f,
                "{}",
                format_err(self.token.line, " at end", &self.message)
            )
        } else {
            write!(
                f,
                "{}",
                format_err(
                    self.token.line,
                    &format!(" at '{}'", self.token.lexeme),
                    &self.message
                )
            )
        }
    }
}

impl std::error::Error for ParseError {}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Parser {
        Parser { tokens, current: 0 }
    }

    pub fn parse(mut self) -> Option<Vec<Stmt>> {
        let mut statements = Vec::new();
        let mut had_error = false;

        while !self.is_at_end() {
            match self.declaration() {
                Ok(statement) => statements.push(statement),
                Err(err) => {
                    had_error = true;
                    println!("{}", err);
                    self.synchronize();
                    if self.is_at_end() {
                        break;
                    }
                }
            }
        }

        if had_error {
            None
        } else {
            Some(statements)
        }
    }

    // Declaration statement is the top-level one, it contains
    // all statements that declare stuff, and also everything else
    fn declaration(&mut self) -> Result<Stmt> {
        if self.match_token(TokenType::Var) {
            self.var_declaration()
        } else if self.match_token(TokenType::Class) {
            self.class_declaration()
        } else if self.match_token(TokenType::Fun) {
            Ok(Stmt::Function(self.function("function")?))
        } else {
            self.statement()
        }
    }

    fn class_declaration(&mut self) -> Result<Stmt> {
        let name = self.consume(TokenType::Identifier, "Expect class name.")?;
        self.consume(TokenType::LeftBrace, "Expect '{' before class body.")?;

        let mut methods = Vec::new();
        while !self.check(TokenType::RightBrace) && !self.is_at_end() {
            methods.push(self.function("method")?);
        }
        self.consume(TokenType::RightBrace, "Expect '}' after class body")?;

        Ok(Stmt::Class { name, methods })
    }

    fn function(&mut self, kind: &'static str) -> Result<StmtFunction> {
        let name = self.consume(TokenType::Identifier, format!("Expect {} name", kind))?;
        self.consume(
            TokenType::LeftParen,
            format!("Expect '(' after {} name.", kind),
        )?;

        let mut params = Vec::new();
        if !self.check(TokenType::RightParen) {
            loop {
                params.push(self.consume(TokenType::Identifier, "Expect paramater name")?);

                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
        }
        self.consume(TokenType::RightParen, "Expect ')' after parameters.")?;
        self.consume(
            TokenType::LeftBrace,
            format!("Expect '{{' before {} body.", kind),
        )?;
        let body = self.block()?;

        Ok(StmtFunction { name, params, body })
    }

    fn var_declaration(&mut self) -> Result<Stmt> {
        let name = self.consume(TokenType::Identifier, "Expect variable name")?;

        let initializer = if self.match_token(TokenType::Equal) {
            Some(self.expression()?)
        } else {
            None
        };

        self.consume(
            TokenType::Semicolon,
            "Expect ';' after variable declaration.",
        )?;
        Ok(Stmt::Var { name, initializer })
    }

    fn statement(&mut self) -> Result<Stmt> {
        if self.match_token(TokenType::If) {
            self.if_statement()
        } else if self.match_token(TokenType::Print) {
            self.print_statement()
        } else if self.match_token(TokenType::Return) {
            self.return_statement()
        } else if self.match_token(TokenType::While) {
            self.while_statement()
        } else if self.match_token(TokenType::For) {
            self.for_statement()
        } else if self.match_token(TokenType::LeftBrace) {
            Ok(Stmt::Block(self.block()?))
        } else {
            self.expression_statement()
        }
    }

    fn return_statement(&mut self) -> Result<Stmt> {
        let keyword = self.previous();
        let value = if !self.check(TokenType::Semicolon) {
            self.expression()?
        } else {
            Expr::Literal(Literal::Nil)
        };

        self.consume(TokenType::Semicolon, "Expect ';' after return value.")?;

        Ok(Stmt::Return { keyword, value })
    }

    fn for_statement(&mut self) -> Result<Stmt> {
        self.consume(TokenType::LeftParen, "Expect '(' after 'for'.")?;
        let initializer = if self.match_token(TokenType::Semicolon) {
            None
        } else if self.match_token(TokenType::Var) {
            Some(self.var_declaration()?)
        } else {
            Some(self.expression_statement()?)
        };

        let condition = if !self.check(TokenType::Semicolon) {
            self.expression()?
        } else {
            Expr::Literal(Literal::Bool(true))
        };
        self.consume(TokenType::Semicolon, "Expect ';' after loop condition.")?;

        let increment = if !self.check(TokenType::RightParen) {
            Some(self.expression()?)
        } else {
            None
        };
        self.consume(TokenType::RightParen, "Expect ')' after for clauses.")?;

        let mut body = self.statement()?;

        if let Some(increment) = increment {
            body = Stmt::Block(vec![body, Stmt::Expression(increment)]);
        }

        body = Stmt::While {
            condition,
            body: Box::new(body),
        };

        if let Some(initializer) = initializer {
            body = Stmt::Block(vec![initializer, body]);
        }

        Ok(body)
    }

    fn while_statement(&mut self) -> Result<Stmt> {
        self.consume(TokenType::LeftParen, "Expect '(' after 'while'")?;
        let condition = self.expression()?;
        self.consume(TokenType::RightParen, "Expect ')' after condition")?;
        let body = Box::new(self.statement()?);

        Ok(Stmt::While { condition, body })
    }

    fn if_statement(&mut self) -> Result<Stmt> {
        self.consume(TokenType::LeftParen, "Expect '(' after 'if'")?;
        let condition = self.expression()?;
        self.consume(TokenType::RightParen, "Expect ')' after if condition")?;

        let then_branch = Box::new(self.statement()?);
        let else_branch = if self.match_token(TokenType::Else) {
            Some(Box::new(self.statement()?))
        } else {
            None
        };

        Ok(Stmt::If {
            condition,
            then_branch,
            else_branch,
        })
    }

    fn block(&mut self) -> Result<Vec<Stmt>> {
        let mut statements = Vec::new();
        while !self.check(TokenType::RightBrace) && !self.is_at_end() {
            statements.push(self.declaration()?);
        }

        self.consume(TokenType::RightBrace, "Expect '}' after block")?;

        Ok(statements)
    }

    fn print_statement(&mut self) -> Result<Stmt> {
        // We have already matched and consumed the print-token
        let value = self.expression()?;

        self.consume(TokenType::Semicolon, "Expect ';' after value")?;

        Ok(Stmt::Print(value))
    }

    fn expression_statement(&mut self) -> Result<Stmt> {
        let expr = self.expression()?;
        self.consume(TokenType::Semicolon, "Expect ';' after expression")?;
        Ok(Stmt::Expression(expr))
    }

    fn expression(&mut self) -> Result<Expr> {
        self.assignment()
    }

    fn assignment(&mut self) -> Result<Expr> {
        // A "hack" because we can't know if it's an assignment or not until
        // we have parsed the name
        // https://craftinginterpreters.com/statements-and-state.html#assignment-syntax

        let expr = self.logic_or()?;
        if self.match_token(TokenType::Equal) {
            let equals = self.previous();
            let value = self.assignment()?;
            if let Expr::Variable { name, .. } = expr {
                return Ok(Expr::Assign {
                    expr_id: next_expr_id(),
                    name,
                    value: Box::new(value),
                });
            } else if let Expr::Get { name, object } = expr {
                return Ok(Expr::Set {
                    object,
                    name,
                    value: Box::new(value),
                });
            }

            println!("{}", ParseError::new(equals, "Invalid assignment target"));
        }

        Ok(expr)
    }

    fn logic_or(&mut self) -> Result<Expr> {
        let mut expr = self.logic_and()?;
        while self.match_token(TokenType::Or) {
            let operator = self.previous();
            let right = self.logic_and()?;
            expr = Expr::Logical {
                left: Box::new(expr),
                operator: operator.clone(),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn logic_and(&mut self) -> Result<Expr> {
        let mut expr = self.equality()?;
        while self.match_token(TokenType::And) {
            let operator = self.previous();
            let right = self.equality()?;
            expr = Expr::Logical {
                left: Box::new(expr),
                operator: operator.clone(),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn equality(&mut self) -> Result<Expr> {
        let mut expr = self.comparison()?;

        while self.match_tokens(&[TokenType::BangEqual, TokenType::EqualEqual]) {
            let operator = self.previous();
            let right = self.comparison()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                operator: operator.clone(),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn comparison(&mut self) -> Result<Expr> {
        let mut expr = self.addition()?;

        while self.match_tokens(&[
            TokenType::Greater,
            TokenType::GreaterEqual,
            TokenType::Less,
            TokenType::LessEqual,
        ]) {
            let operator = self.previous();
            let right = self.addition()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                operator: operator.clone(),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn addition(&mut self) -> Result<Expr> {
        let mut expr = self.multiplication()?;

        while self.match_tokens(&[TokenType::Minus, TokenType::Plus]) {
            let operator = self.previous();
            let right = self.multiplication()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                operator: operator.clone(),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn multiplication(&mut self) -> Result<Expr> {
        let mut expr = self.unary()?;

        while self.match_tokens(&[TokenType::Slash, TokenType::Star]) {
            let operator = self.previous();
            let right = self.unary()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                operator: operator.clone(),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn unary(&mut self) -> Result<Expr> {
        if self.match_tokens(&[TokenType::Bang, TokenType::Minus]) {
            let operator = self.previous();
            let right = self.unary()?;
            return Ok(Expr::Unary {
                operator: operator.clone(),
                right: Box::new(right),
            });
        }
        Ok(self.call()?)
    }

    fn call(&mut self) -> Result<Expr> {
        let mut expr = self.primary()?;
        loop {
            if self.match_token(TokenType::LeftParen) {
                expr = self.finish_call(expr)?;
            } else if self.match_token(TokenType::Dot) {
                let name = self.consume(TokenType::Identifier, "Expect property name after '.'")?;
                expr = Expr::Get {
                    object: Box::new(expr),
                    name,
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn finish_call(&mut self, callee: Expr) -> Result<Expr> {
        let mut arguments = Vec::new();
        if !self.check(TokenType::RightParen) {
            loop {
                arguments.push(self.expression()?);
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
        }

        let paren = self.consume(TokenType::RightParen, "Expect ')' after arguments.")?;

        Ok(Expr::Call {
            callee: Box::new(callee),
            paren,
            arguments,
        })
    }

    fn primary(&mut self) -> Result<Expr> {
        use TokenType::*;
        let next_token = self.advance();
        Ok(match &next_token.typ {
            False => Expr::Literal(Literal::Bool(false)),
            True => Expr::Literal(Literal::Bool(true)),
            Nil => Expr::Literal(Literal::Nil),
            Number(number) => Expr::Literal(Literal::Number(*number)),
            String(string) => Expr::Literal(Literal::String(string.clone())),
            LeftParen => {
                let expr = self.expression()?;
                self.consume(RightParen, "Expect ')' after expression")?;
                Expr::Grouping(Box::new(expr))
            }
            Identifier => Expr::Variable {
                expr_id: next_expr_id(),
                name: self.previous(),
            },
            // NOTE: In the book, this will not advance the parsing
            _ => Err(ParseError::new(
                next_token,
                "Expected expression".to_owned(),
            ))?,
        })
    }

    fn consume(&mut self, typ: TokenType, message: impl Into<String>) -> Result<Token> {
        if self.check(typ) {
            Ok(self.advance())
        } else {
            Err(ParseError::new(self.peek().clone(), message.into()))
        }
    }

    fn synchronize(&mut self) {
        use TokenType::*;
        self.advance();

        while !self.is_at_end() {
            if self.previous().typ == TokenType::Semicolon {
                return;
            }

            if let Class | Fun | Var | For | If | While | Print | Return = self.peek().typ {
                return;
            }
        }
    }

    fn match_token(&mut self, typ: TokenType) -> bool {
        if self.check(typ) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn match_tokens(&mut self, types: &[TokenType]) -> bool {
        for typ in types {
            if self.check(typ.clone()) {
                self.advance();
                return true;
            }
        }

        return false;
    }

    fn check(&self, typ: TokenType) -> bool {
        if self.is_at_end() {
            false
        } else {
            self.peek().typ == typ
        }
    }

    fn advance(&mut self) -> Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn is_at_end(&self) -> bool {
        self.peek().typ == TokenType::EOF
    }

    fn peek(&self) -> &Token {
        self.tokens
            .get(self.current)
            .as_ref()
            .expect("Peek called when we have run out of tokens")
    }

    fn previous(&self) -> Token {
        self.tokens
            .get(self.current - 1)
            .expect("Out of index in previous")
            .clone()
    }
}
