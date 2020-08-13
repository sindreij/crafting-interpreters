pub struct Scanner<'a> {
    start: &'a str,
    // How many characters into "start"" are we currently
    current: usize,
    line: usize,
}

#[derive(Copy, Clone, Debug)]
pub struct Token<'a> {
    pub typ: TokenType,
    pub str: &'a str,
    pub line: usize,
}

#[derive(Debug, Copy, Clone, PartialEq)]
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
    NOOP,
    Error,
}

impl std::fmt::Display for TokenType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("{:?}", self))
    }
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
        use TokenType::*;

        self.skip_whitespace();

        self.start = &self.start[self.current..];
        self.current = 0;

        if self.is_at_end() {
            return self.make_token(EOF);
        }

        let c = self.advance();

        match c {
            '(' => self.make_token(LeftParen),
            ')' => self.make_token(RightParen),
            '{' => self.make_token(LeftBrace),
            '}' => self.make_token(RightBrace),
            ';' => self.make_token(Semicolon),
            ',' => self.make_token(Comma),
            '.' => self.make_token(Dot),
            '-' => self.make_token(Minus),
            '+' => self.make_token(Plus),
            '/' => self.make_token(Slash),
            '*' => self.make_token(Star),

            '!' if self.next_match('=') => self.make_token(BangEqual),
            '!' => self.make_token(Bang),

            '=' if self.next_match('=') => self.make_token(EqualEqual),
            '=' => self.make_token(Equal),

            '<' if self.next_match('=') => self.make_token(LessEqual),
            '<' => self.make_token(Less),

            '>' if self.next_match('=') => self.make_token(TokenType::GreaterEqual),
            '>' => self.make_token(Greater),

            '"' => self.string(),

            c if c.is_digit(10) => self.number(),
            c if is_alpha(c) => self.identifier(),
            _ => self.error_token("Unexpected character."),
        }
    }

    fn identifier(&mut self) -> Token<'a> {
        while check_op(|c| is_alpha(c) || c.is_digit(10), self.peek()) {
            self.advance();
        }

        self.make_token(self.identifier_type())
    }

    fn identifier_type(&self) -> TokenType {
        match self.char_at(0) {
            'a' => self.check_keyword(1, 2, "nd", TokenType::And),
            'c' => self.check_keyword(1, 4, "lass", TokenType::Class),
            'e' => self.check_keyword(1, 3, "lse", TokenType::Else),
            'i' => self.check_keyword(1, 1, "f", TokenType::If),
            'n' => self.check_keyword(1, 2, "il", TokenType::Nil),
            'o' => self.check_keyword(1, 1, "r", TokenType::Or),
            'p' => self.check_keyword(1, 4, "rint", TokenType::Print),
            'r' => self.check_keyword(1, 5, "eturn", TokenType::Return),
            's' => self.check_keyword(1, 4, "uper", TokenType::Super),
            'v' => self.check_keyword(1, 2, "ar", TokenType::Var),
            'w' => self.check_keyword(1, 4, "hile", TokenType::While),
            'f' if self.current > 1 => match self.char_at(1) {
                'a' => self.check_keyword(2, 3, "lse", TokenType::False),
                'o' => self.check_keyword(2, 1, "r", TokenType::For),
                'u' => self.check_keyword(2, 1, "n", TokenType::Fun),
                _ => TokenType::Identifier,
            },
            't' if self.current > 1 => match self.char_at(1) {
                'h' => self.check_keyword(2, 2, "is", TokenType::This),
                'r' => self.check_keyword(2, 2, "ue", TokenType::True),
                _ => TokenType::Identifier,
            },
            _ => TokenType::Identifier,
        }
    }

    fn check_keyword(&self, start: usize, length: usize, rest: &str, typ: TokenType) -> TokenType {
        if self.current == start + length && &self.start[start..start + length] == rest {
            typ
        } else {
            TokenType::Identifier
        }
    }

    fn number(&mut self) -> Token<'a> {
        while op_is_digit(self.peek()) {
            self.advance();
        }

        // Look for a fractional part.
        if self.peek() == Some('.') && op_is_digit(self.peek_next()) {
            // Consume the "."
            self.advance();

            while op_is_digit(self.peek()) {
                self.advance();
            }
        }

        self.make_token(TokenType::Number)
    }

    fn string(&mut self) -> Token<'a> {
        while self.peek() != Some('"') && !self.is_at_end() {
            if self.peek() == Some('\n') {
                self.line += 1;
            }
            self.advance();
        }

        if self.is_at_end() {
            return self.error_token("Unterminated string.");
        }

        // The closing quote
        assert_eq!(self.advance(), '"');

        self.make_token(TokenType::String)
    }

    fn skip_whitespace(&mut self) {
        loop {
            match self.peek() {
                Some('/') if self.peek_next() == Some('/') => {
                    // A comment goes until the end of the line.
                    while self.peek() != Some('\n') && !self.is_at_end() {
                        self.advance();
                    }
                }
                Some('\n') => {
                    self.line += 1;
                    self.advance();
                }
                Some(c) if c.is_whitespace() => {
                    self.advance();
                }
                _ => {
                    return;
                }
            }
        }
    }

    fn peek(&self) -> Option<char> {
        if self.is_at_end() {
            None
        } else {
            Some(self.char_at(self.current))
        }
    }
    fn peek_next(&self) -> Option<char> {
        if self.is_at_end() {
            None
        } else {
            Some(self.char_at(self.current + 1))
        }
    }

    fn advance(&mut self) -> char {
        self.current += 1;
        self.char_at(self.current - 1)
    }

    fn char_at(&self, idx: usize) -> char {
        self.start
            .chars()
            .nth(idx)
            .expect("char_at called with out of index number")
    }

    fn next_match(&mut self, expected: char) -> bool {
        if self.is_at_end() {
            false
        } else if self.char_at(self.current) != expected {
            false
        } else {
            self.current += 1;
            true
        }
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

fn op_is_digit(op: Option<char>) -> bool {
    match op {
        Some(c) => c.is_digit(10),
        None => false,
    }
}

fn is_alpha(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn check_op(fun: fn(char) -> bool, op: Option<char>) -> bool {
    match op {
        Some(c) => fun(c),
        None => false,
    }
}
