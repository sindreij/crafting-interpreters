use phf::phf_map;

use crate::{
    error_reporter::ErrorReporter,
    token::{Token, TokenType},
};

static KEYWORDS: phf::Map<&'static str, TokenType> = phf_map! {
    "and" => TokenType::And,
    "class" => TokenType::Class,
    "else"=> TokenType::Else,
    "false"=> TokenType::False,
    "for"=> TokenType::For,
    "fun"=> TokenType::Fun,
    "if"=> TokenType::If,
    "nil"=> TokenType::Nil,
    "or"=> TokenType::Or,
    "print"=> TokenType::Print,
    "return"=> TokenType::Return,
    "super"=> TokenType::Super,
    "this"=> TokenType::This,
    "true"=> TokenType::True,
    "var"=> TokenType::Var,
    "while"=> TokenType::While
};

pub struct Scanner<'a> {
    source: Vec<char>,
    tokens: Vec<Token>,
    start: usize,
    current: usize,
    line: u32,
    errors: &'a mut ErrorReporter,
}

impl<'a> Scanner<'a> {
    pub fn new(source: &'a str, errors: &'a mut ErrorReporter) -> Scanner<'a> {
        Scanner {
            source: source.chars().collect(),
            tokens: vec![],
            start: 0,
            current: 0,
            line: 1,
            errors,
        }
    }

    pub fn scan_tokens(&mut self) -> Vec<Token> {
        while !self.is_at_end() {
            self.start = self.current;
            self.scan_token();
        }

        self.tokens
            .push(Token::new(TokenType::EOF, String::new(), self.line));
        self.tokens.clone()
    }

    fn scan_token(&mut self) {
        use TokenType::*;
        let c = self.advance();

        match c {
            '(' => self.add_token(LeftParen),
            ')' => self.add_token(RightParen),
            '{' => self.add_token(LeftBrace),
            '}' => self.add_token(RightBrace),
            ',' => self.add_token(Comma),
            '.' => self.add_token(Dot),
            '-' => self.add_token(Minus),
            '+' => self.add_token(Plus),
            ';' => self.add_token(Semicolon),
            '*' => self.add_token(Star),
            '!' if self.match_next('=') => self.add_token(BangEqual),
            '!' => self.add_token(Bang),
            '=' if self.match_next('=') => self.add_token(EqualEqual),
            '=' => self.add_token(Equal),
            '<' if self.match_next('=') => self.add_token(LessEqual),
            '<' => self.add_token(Less),
            '>' if self.match_next('=') => self.add_token(GreaterEqual),
            '>' => self.add_token(Greater),
            '/' if self.match_next('/') => {
                while self.peek() != '\n' && !self.is_at_end() {
                    self.advance();
                }
            }
            '/' => self.add_token(Slash),
            ' ' | '\r' | '\t' => {
                // ignore whitespace
            }
            '\n' => {
                self.line += 1;
            }
            '"' => self.string(),
            '0'..='9' => self.number(),
            'a'..='z' | 'A'..='Z' | '_' => self.identifier(),

            unknown => self
                .errors
                .error(self.line, format!("Unexpected character {}", unknown)),
        }
    }

    fn identifier(&mut self) {
        while self.peek().is_ascii_alphanumeric() {
            self.advance();
        }

        let text = self.source[self.start..self.current]
            .iter()
            .collect::<String>();
        let typ = if let Some(typ) = KEYWORDS.get(text.as_str()) {
            typ.clone()
        } else {
            TokenType::Identifier
        };

        self.add_token(typ);
    }

    fn number(&mut self) {
        while self.peek().is_digit(10) {
            self.advance();
        }

        // Look for a fractional part
        if self.peek() == '.' && self.peek_next().is_digit(10) {
            // Consume the "."
            self.advance();

            while self.peek().is_digit(10) {
                self.advance();
            }
        }

        self.add_token(TokenType::Number(
            self.source[self.start..self.current]
                .iter()
                .collect::<String>()
                .parse()
                .expect("Error parsing number as f64"),
        ))
    }

    fn string(&mut self) {
        while self.peek() != '"' && !self.is_at_end() {
            if self.peek() == '\n' {
                self.line += 1;
            }
            self.advance();
        }

        // Unterminated string.
        if self.is_at_end() {
            self.errors
                .error(self.line, "Unterminated string".to_owned());
            return;
        }

        // The closing "
        self.advance();

        let value = self.source[self.start + 1..self.current - 1]
            .iter()
            .collect();
        self.add_token(TokenType::String(value));
    }

    fn advance(&mut self) -> char {
        self.current += 1;
        self.source[self.current - 1]
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            return '\0';
        }
        self.source[self.current]
    }

    fn peek_next(&self) -> char {
        if self.current + 1 >= self.source.len() {
            return '\0';
        }
        self.source[self.current + 1]
    }

    fn match_next(&mut self, expected: char) -> bool {
        if self.is_at_end() {
            return false;
        }

        if self.source[self.current] != expected {
            return false;
        }

        self.current += 1;
        true
    }

    fn add_token(&mut self, typ: TokenType) {
        let text: String = self.source[self.start..self.current].iter().collect();
        self.tokens.push(Token::new(typ, text, self.line))
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }
}
