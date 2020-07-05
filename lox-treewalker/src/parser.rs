use crate::{
    ast::{Expr, Literal, Stmt},
    error_reporter::format_err,
    token::{Token, TokenType},
};

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
    fn new(token: Token, message: String) -> ParseError {
        ParseError { token, message }
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
        if self.match_token(&[&TokenType::Var]) {
            self.var_declaration()
        } else {
            self.statement()
        }
    }

    fn var_declaration(&mut self) -> Result<Stmt> {
        let name = self.consume(&TokenType::Identifier, "Expect variable name")?;

        let initializer = if self.match_token(&[&TokenType::Equal]) {
            Some(self.expression()?)
        } else {
            None
        };

        self.consume(
            &TokenType::Semicolon,
            "Expect ';' after variable declaration.",
        )?;
        Ok(Stmt::Var { name, initializer })
    }

    fn statement(&mut self) -> Result<Stmt> {
        if self.match_token(&[&TokenType::Print]) {
            self.print_statement()
        } else {
            self.expression_statement()
        }
    }

    fn print_statement(&mut self) -> Result<Stmt> {
        // We have already matched and consumed the print-token
        let value = self.expression()?;

        self.consume(&TokenType::Semicolon, "Expect ';' after value")?;

        Ok(Stmt::Print(value))
    }

    fn expression_statement(&mut self) -> Result<Stmt> {
        let expr = self.expression()?;
        self.consume(&TokenType::Semicolon, "Expect ';' after expression")?;
        Ok(Stmt::Expression(expr))
    }

    fn expression(&mut self) -> Result<Expr> {
        self.equality()
    }

    fn equality(&mut self) -> Result<Expr> {
        let mut expr = self.comparison()?;

        while self.match_token(&[&TokenType::BangEqual, &TokenType::EqualEqual]) {
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

        while self.match_token(&[
            &TokenType::Greater,
            &TokenType::GreaterEqual,
            &TokenType::Less,
            &TokenType::LessEqual,
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

        while self.match_token(&[&TokenType::Minus, &TokenType::Plus]) {
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

        while self.match_token(&[&TokenType::Slash, &TokenType::Star]) {
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
        if self.match_token(&[&TokenType::Bang, &TokenType::Minus]) {
            let operator = self.previous();
            let right = self.unary()?;
            return Ok(Expr::Unary {
                operator: operator.clone(),
                right: Box::new(right),
            });
        }
        Ok(self.primary()?)
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
                self.consume(&RightParen, "Expect ')' after expression")?;
                Expr::Grouping(Box::new(expr))
            }
            Identifier => Expr::Variable {
                name: self.previous(),
            },
            // NOTE: In the book, this will not advance the parsing
            _ => Err(ParseError::new(
                next_token,
                "Expected expression".to_owned(),
            ))?,
        })
    }

    fn consume(&mut self, typ: &TokenType, message: &str) -> Result<Token> {
        if self.check(typ) {
            Ok(self.advance())
        } else {
            Err(ParseError::new(self.peek().clone(), message.to_owned()))
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

    fn match_token(&mut self, types: &[&TokenType]) -> bool {
        for typ in types {
            if self.check(typ) {
                self.advance();
                return true;
            }
        }

        return false;
    }

    fn check(&self, typ: &TokenType) -> bool {
        if self.is_at_end() {
            false
        } else {
            &self.peek().typ == typ
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
