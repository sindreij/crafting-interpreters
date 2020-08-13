pub struct Scanner<'a> {
    start: &'a str,
    // How many characters into "start"" are we currently
    current: usize,
    line: usize,
}

pub struct Token<'a> {
    pub typ: TokenType,
    pub str: &'a str,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Single-character tokens.
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,

    // One or two character tokens.
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,

    // Literals.
    Identifier,
    String,
    Number,

    // Keywords.
    And,
    Class,
    Else,
    False,
    Fun,
    For,
    If,
    Nil,
    Or,
    Print,
    Return,
    Super,
    This,
    True,
    Var,
    While,

    EOF,
    Error,
}

impl<'a> Scanner<'a> {
    pub fn new(source: &'a str) -> Scanner<'a> {
        Scanner {
            start: source,
            current: 0,
            line: 1,
        }
    }

    pub fn scan_token(&mut self) -> Token<'a> {
        self.start = &self.start[self.current..];
        self.current = 0;

        if self.is_at_end() {
            return self.make_token(TokenType::EOF);
        }

        self.current += 1;
        return self.error_token("Unexpected character.");
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.start.len()
    }

    fn make_token(&self, typ: TokenType) -> Token<'a> {
        Token {
            typ,
            str: &self.start[..self.current],
            line: self.line,
        }
    }
    fn error_token(&self, message: &'static str) -> Token<'static> {
        Token {
            typ: TokenType::Error,
            str: message,
            line: self.line,
        }
    }
}
